use crate::{core::Symbol, watcher::Watcher};
use indicatif::ProgressBar;

use std::fmt::Display;

use itertools::Itertools;
use log::{debug, error, info, log_enabled, trace};

use crate::{
    core::{
        Assignment, Clause, ClauseStorage, Evaluation, Lemma, Literal, MaybeConflict, RefLemma,
    },
    Features,
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

pub fn validate(clauses: Vec<Clause>, lemmas: Vec<Lemma>, features: Features) -> Verdict {
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

    info!("populating watchlist and watchtracker");
    let watcher = Watcher::new(&clause_db);

    info!("assigning units from initial formula");
    let mut assignment = Assignment::new();
    for lit in clause_db.clauses().filter_map(|(_, clause)| clause.unit()) {
        if assignment.conflicts(lit) {
            error!("prepropagation yields conflict on ({})", lit);
            return Verdict::EarlyRefutation;
        }
        assignment.assign(lit);
    }

    if let MaybeConflict::Conflict = propagate(&clause_db, &watcher, &mut assignment) {
        error!("prepropagation yields conflict");
        return Verdict::EarlyRefutation;
    }
    debug!("prepropagation result ({})", assignment);

    let validator = Validator {
        clause_db,
        watcher,
        assignment,
    };

    validator.forward_validate(&lemmas, features)
}

impl Validator {
    /// Sequentially validate each lemma by checking if it has the RUP property.
    /// Clauses are added and removed from the clause database during this process.
    fn forward_validate(mut self, lemmas: &[RefLemma], features: Features) -> Verdict {
        info!("forward validating only");

        let bar = if features.progress {
            ProgressBar::new(lemmas.len() as u64)
        } else {
            ProgressBar::hidden()
        };

        for lemma in lemmas {
            // verify each lemma in order
            match lemma {
                RefLemma::Deletion(c_ref) => {
                    // TODO find out how to properly identify unit clauses and
                    // ignore their deletion
                    if let Some(clause) = self.clause_db.get_clause(*c_ref) {
                        if clause.is_unit(&self.assignment) {
                            debug!("is unit clause, skipping deletion ({})", clause);
                        } else {
                            trace!("delete ({})", clause);
                            self.clause_db.del_clause(*c_ref);
                        }
                    } else {
                        error!(
                            "tried delete but did not exist ({})",
                            self.clause_db.get_any_clause(*c_ref)
                        );
                    }
                }
                RefLemma::Addition(c_ref) => {
                    let clause = self.clause_db.get_any_clause(*c_ref);
                    debug!("checking ({})", clause);

                    // check if the lemma being added is redundant by first checking
                    // whether it is RUP, and if that doesn't work check if it is
                    // RAT
                    if self.rup(clause)
                        || (!features.rup_only && check_rat(&self.clause_db, clause))
                    {
                        if clause.is_empty() {
                            debug!("verified the empty clause, refutation validated");
                            return Verdict::RefutationVerified;
                        }
                        debug!("is redundant ({})", clause);
                        if let Some(unit) = clause.unit() {
                            // if we add a unit, add it to the assignment after
                            // it is verified.
                            // this may already conflict with the assignment,
                            // but we don't check it since the lemma is RUP
                            debug!("found unit in proof {}", unit);
                            self.assignment.assign(unit);
                        }

                        self.clause_db.activate_clause(*c_ref);
                    } else {
                        error!("lemma not redundant, proof refuted ({})", clause);
                        return Verdict::RefutationRefuted;
                    }

                    trace!("propagating from ({})", self.assignment);
                    propagate(&self.clause_db, &mut self.watcher, &mut self.assignment);
                    trace!("propagating to ({})", self.assignment);
                }
            }

            bar.inc(1);
        }

        // if we have not seen the empty clause yet and it is not RUP, then the
        // proof does not show a conflict and therefore there is no refutation to
        // verify
        error!("no conflict detected or empty clause present");
        Verdict::NoConflict
    }

    /// Check if the given clause has the rup property
    fn rup(&self, lemma: &Clause) -> bool {
        // construct a new assignment by adding all the negated literals from
        // the lemma as units
        let mut assignment = self.assignment.clone();
        for lit in lemma.literals() {
            if assignment.conflicts(!lit) {
                trace!("inverting lemma lead to conflict on {} in ({})", lit, lemma);
                return true;
            }
            assignment.assign(!lit);
        }

        // try to propagate the assignment
        match propagate(&self.clause_db, &self.watcher, &mut assignment) {
            MaybeConflict::Conflict => true,
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

    let mut to_check = assignment.literals().copied().collect_vec();

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
        for c_ref in watcher.watched_by(Symbol::from(literal)) {
            if !clause_db.is_active(c_ref) {
                continue;
            }

            // get both the literals that this clause is watching
            if let Some((lit1, mut lit2)) = watcher.watches(c_ref) {
                // check if one of the watched literals is satisfied
                if assignment.has_literal(lit1) || assignment.has_literal(lit2) {
                    // if the clause is satisfied, ignore and keep them on the
                    // same watchlists.
                    trace!(
                        "skipping satisfied clause ({}) via {} or {}",
                        clause_db.get_clause(c_ref).unwrap(),
                        lit1,
                        lit2
                    );
                    continue;
                }

                // swap around the literals to make sure that lit2 is the
                // known literal.
                if Symbol::from(lit1) == Symbol::from(literal) {
                    lit2 = lit1;
                }

                // one of the two literals must be the negation of the literal
                // try to find the next unassigned literal to watch instead
                // if none can be found this is a unit

                // unwatch the literal we just checked
                // this step also
                if let Some(unit) = watcher.update(
                    c_ref,
                    // lit2 is the known literal here that we just checked
                    Symbol::from(lit2),
                    &clause_db,
                    &assignment,
                ) {
                    // found a unit
                    trace!(
                        "found unit {} in clause ({}), adding  to assignment",
                        unit,
                        clause_db.get_clause(c_ref).unwrap(),
                    );
                    if assignment.conflicts(unit) {
                        // if the assignment results in a conflict, make
                        // sure to report that
                        trace!("encountered conflict");
                        trace!("after: {}", assignment);
                        return MaybeConflict::Conflict;
                    }
                    if assignment.assign(unit) {
                        // if we successfully assigned something new to the
                        // assignment, add it to the list of literals to check
                        trace!("added {}, now ({})", unit, assignment);
                        to_check.push(unit);
                    }
                } else {
                    if log_enabled!(log::Level::Trace) {
                        let (left, right) = watcher.watches(c_ref).unwrap();
                        trace!(
                            "({}) now watches new {} and {}",
                            clause_db.get_clause(c_ref).unwrap(),
                            left,
                            right
                        );
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
