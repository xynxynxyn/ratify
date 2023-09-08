mod watcher;
use indicatif::ProgressBar;
use watcher::Watcher;

use std::fmt::Display;

use itertools::Itertools;
use log::{debug, error, info, trace};

use crate::core::{
    Assignment, Clause, ClauseStorage, Evaluation, Lemma, Literal, MaybeConflict, RefLemma,
};

/// Validator struct that tracks the state of the validation process.
struct Validator {
    /// Storage for all clauses
    clause_db: ClauseStorage,
    /// Watcher keeps track of watched literal responsibilities and
    /// functioniality
    watcher: Watcher,
    /// The current assignment
    assignment: Assignment,
}

pub fn validate(clauses: Vec<Clause>, lemmas: Vec<Lemma>) -> Verdict {
    // determine the upper limit of clauses. this may be significantly larger
    // than the actual number of clauses depending on the number of deletions.
    let clause_count = clauses.len() + lemmas.len();
    let mut clause_db = ClauseStorage::with_capacity(clause_count);

    // add all the initial clauses, this may introduce duplicates. This is not
    // an issue however as the duplicates from the lemma should be marked as
    // inactive.
    clause_db.add_from_iter(clauses.into_iter(), true);

    // convert the lemmas into reflemmas, the same as before but they contain a
    // clause reference instead of a clause.
    let lemmas = lemmas
        .into_iter()
        .map(|lemma| match lemma {
            Lemma::Addition(c) => RefLemma::Addition(clause_db.add_clause(c, false)),
            Lemma::Deletion(c) => RefLemma::Deletion(clause_db.add_clause(c, false)),
        })
        .collect_vec();

    debug!("clause database:\n{}", clause_db.dump());

    let watcher = Watcher::init(&clause_db);

    let validator = Validator {
        clause_db,
        watcher,
        assignment: Assignment::new(),
    };

    validator.forward_validate(&lemmas)
}

impl Validator {
    /// Sequentially validate each lemma by checking if it has the RUP property.
    /// Clauses are added and removed from the clause database during this process.
    fn forward_validate(mut self, lemmas: &[RefLemma]) -> Verdict {
        info!("forward validating rup only");
        let mut empty_clause = false;

        let bar = ProgressBar::new(lemmas.len() as u64);

        for lemma in lemmas {
            // verify each lemma in order
            match lemma {
                RefLemma::Deletion(c_ref) => {
                    // TODO find out how to properly identify unit clauses and
                    // ignore their deletion
                    if let Some(clause) = self.clause_db.get_clause(*c_ref) {
                        if clause.is_unit(&self.assignment) {
                            debug!("skipping unit clause deletion for ({})", clause);
                        } else {
                            debug!("deleted clause ({})", clause);
                            self.clause_db.del_clause(*c_ref);
                        }
                    } else {
                        debug!(
                            "tried deleting clause, but did not exist {}",
                            self.clause_db.get_any_clause(*c_ref)
                        );
                    }
                }
                RefLemma::Addition(c_ref) => {
                    let clause = self.clause_db.get_any_clause(*c_ref);
                    debug!("checking lemma ({})", clause);
                    // check if we encountered the empty clause
                    if clause.is_empty() {
                        empty_clause = true;
                    }

                    // check if the lemma being added is redundant by first checking
                    // whether it is RUP, and if that doesn't work check if it is
                    // RAT
                    //if check_rup(&self.clause_db, clause) {
                    if self.rup(clause) || check_rat(&self.clause_db, clause) {
                        debug!(
                            "lemma is RUP or RAT, extending clause database with ({})",
                            clause
                        );
                        if let Some(unit) = clause.unit() {
                            // if we add a unit, add it to the assignment after
                            // it is verified.
                            if let MaybeConflict::Conflict = self.assignment.assign(unit) {
                                // return early if this does
                                return Verdict::EarlyRefutation;
                            }
                        }

                        self.clause_db.activate_clause(*c_ref);
                    } else {
                        debug!("lemma is neither RUP nor RAT, refuting proof");
                        return Verdict::RefutationRefuted;
                    }

                    propagate(&self.clause_db, &mut self.watcher, &mut self.assignment);
                }
            }

            bar.inc(1);
        }

        bar.finish();

        // if we have not seen the empty clause yet and it is not RUP, then the
        // proof does not show a conflict and therefore there is no refutation to
        // verify
        if !empty_clause && !check_rup(&self.clause_db, &Clause::empty()) {
            error!("no conflict detected");
            Verdict::NoConflict
        } else {
            info!("refutation verified");
            Verdict::RefutationVerified
        }
    }

    /// Check if the given clause has the rup property
    fn rup(&self, lemma: &Clause) -> bool {
        // construct a new assignment by adding all the negated literals from
        // the lemma as units
        let mut assignment = self.assignment.clone();
        for lit in lemma.literals() {
            if let MaybeConflict::Conflict = assignment.assign(!lit) {
                return true;
            }
        }

        // try to propagate the assignment
        match propagate(&self.clause_db, &self.watcher, &mut assignment) {
            MaybeConflict::Conflict => {
                debug!("({}) is RUP", lemma);
                true
            }
            MaybeConflict::NoConflict => false,
        }
    }
}

/// Apply unit propagation and update the given assignment correspondingly.
fn propagate(
    clause_db: &ClauseStorage,
    watcher: &Watcher,
    assignment: &mut Assignment,
) -> MaybeConflict {
    // non core unit propagation
    trace!("applying unit propagation, before: ({})", assignment);
    // keep track of how many literals we processed
    let mut processed = 0;

    let mut to_check = assignment.literals().cloned().collect_vec();

    loop {
        if to_check.len() <= processed {
            // if there are no more literals left and no conflict has been
            // found return that there is no conflict
            trace!("processed entire assignment, after: ({})", assignment);
            return MaybeConflict::NoConflict;
        }

        // Get the first unprocessed literal
        let literal = to_check[processed];
        trace!("processing literal {}", literal);
        processed += 1;

        // go through all the clauses which watch the current literal
        // we have to collect into a vec here because we mutate the watcher
        // inside the loop.
        for c_ref in watcher.watched_by(literal) {
            // get both the literals that this clause is watching
            if let Some((lit1, mut lit2)) = watcher.watches(c_ref) {
                if lit1 == literal || lit2 == literal {
                    // if the clause is satisfied, ignore and keep them on the
                    // same watchlists.
                    continue;
                }

                // swap around the literals to make sure that lit2 is the
                // known literal.
                if lit1.equal(&literal) {
                    lit2 = lit1;
                }

                // one of the two literals must be the negation of the literal
                // try to find the next unassigned literal to watch instead
                // if none can be found this is a unit

                // unwatch the literal we just checked
                // this step also
                if let Some(unit) = watcher.unwatch_and_watch(
                    c_ref,
                    // lit2 is the known literal here that we just checked
                    lit2,
                    &clause_db,
                    &assignment,
                ) {
                    // found a unit
                    trace!("found a unit {}, adding to assignment", unit);
                    if let MaybeConflict::Conflict = assignment.assign(unit) {
                        // if the assignment results in a conflict, make
                        // sure to report that
                        trace!("encountered conflict");
                        trace!("after: {}", assignment);
                        return MaybeConflict::Conflict;
                    } else {
                        // if we successfully assigned something new to the
                        // assignment, add it to the list
                        // the given literal should not have been assigned
                        // already, otherwise that should have been cought by
                        // the satisfiability check earlier
                        to_check.push(unit);
                    }
                }
            }
        }
    }
}

/// The end result of checking a proof against a formula.
pub enum Verdict {
    /// The refutation has successfully been validated.
    RefutationVerified,
    /// The refutation could not be validated as the clauses are not redundant.
    RefutationRefuted,
    /// Returned if a refutation is encountered before the empty clause is
    /// checked.
    EarlyRefutation,
    /// The proof does not yield an empty clause. Therefore, there is no
    /// refutation to validate.
    NoConflict,
}

impl Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::RefutationVerified => write!(f, "s VERIFIED"),
            _ => write!(f, "s NOT VERIFIED"),
        }
    }
}

/// Verify whether the given clause is RUP with regards to all clauses in the
/// database.
/// 1. Create an assignment from the lemma by negating the literals.
/// 2. Evaluate each clause in the database against the assignment.
/// 3. If a single clause is false, RUP is verified.
/// 4. If a unit is encountered
fn check_rup(clause_db: &ClauseStorage, lemma: &Clause) -> bool {
    trace!("checking RUP property for lemma ({})", lemma);
    // create a new assignment from the lemma
    let mut assignment = Assignment::from(lemma);
    // track whether the assignment has been modified
    let mut modified = false;
    loop {
        for (_, clause) in clause_db.clauses() {
            match clause.eval(&assignment) {
                // if any clause evals to false then RUP is verified
                Evaluation::False => {
                    return true;
                }
                // if a unit is found, extend the assignment
                Evaluation::Unit(lit) => {
                    assignment.assign(lit);
                    modified = true;
                }
                _ => (),
            }
        }

        if modified {
            modified = false;
        } else {
            // if no units were found and no false clauses exist, RUP is not
            // validated
            return false;
        }
    }
}

fn check_rat_on(clause_db: &ClauseStorage, lemma: &Clause, lit: Literal) -> bool {
    trace!(
        "checking RAT property for lemma ({}) on literal ({})",
        lemma,
        lit
    );
    // find all clauses in the database which contain the
    // negation of the literal.
    for (_, clause) in clause_db.clauses().filter(|(_, c)| c.has_literal(!lit)) {
        // create the resolvent
        let resolvent = lemma.resolve(clause, lit);
        // check for rup property
        if !check_rup(clause_db, &resolvent) {
            trace!("RAT verification on ({}) failed", lit);
            return false;
        }
    }
    // if all of the resolvents are RUP the lemma is RAT
    true
}
/// Check if a lemma has the RAT property with respect to the provided clause
/// database.
fn check_rat(clause_db: &ClauseStorage, lemma: &Clause) -> bool {
    trace!("checking RAT property for lemma '{}'", lemma);
    // check RAT property for each pivot literal
    for lit in lemma.literals() {
        if check_rat_on(clause_db, lemma, *lit) {
            return true;
        }
    }

    trace!("RAT verification failed");
    false
}
