use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use log::{debug, error, info, trace};

use crate::core::{Assignment, Clause, ClauseStorage, Evaluation, Lemma, Literal};

/// Validator struct that tracks the state of the validation process.
struct Validator<'a> {
    /// Storage for all clauses
    clause_db: ClauseStorage,
    /// Watchlist for unit propagation
    watchlist: HashMap<Literal, Vec<&'a Clause>>,
    /// The current assignment
    assignment: HashSet<Literal>,
}

pub fn validate(clauses: Vec<Clause>, lemmas: Vec<Lemma>) -> Verdict {
    let lemma_clauses = lemmas.iter().filter_map(|lemma| {
        if let Lemma::Addition(c) = lemma {
            Some(c.clone())
        } else {
            None
        }
    });

    let lemma_clause_count = match lemma_clauses.size_hint() {
        (_, Some(size)) => size,
        (size, _) => size,
    };
    info!(
        "constructing solver with {} active and {} passive clauses",
        clauses.len(),
        lemma_clause_count
    );
    let mut clause_db = ClauseStorage::with_capacity(clauses.len() + lemma_clause_count);
    clause_db.add_from_iter(clauses.into_iter(), true);
    clause_db.add_from_iter(lemma_clauses, false);

    let validator = Validator {
        clause_db,
        watchlist: HashMap::new(),
        assignment: HashSet::new(),
    };

    validator.forward_validate(&lemmas)
}

impl Validator<'_> {
    /// Sequentially validate each lemma by checking if it has the RUP property.
    /// Clauses are added and removed from the clause database during this process.
    fn forward_validate(mut self, lemmas: &[Lemma]) -> Verdict {
        info!("forward validating rup only");
        let mut empty_clause = false;
        let mut assignment = Assignment::new();
        propagate(&self.clause_db, &mut assignment);

        let mut processed = 0;
        let log_cutoff = 100;
        let max_lemmas = lemmas.len();

        for lemma in lemmas {
            // verify each lemma in order
            match lemma {
                Lemma::Deletion(c) => {
                    // TODO find out how to properly identify unit clauses and
                    // ignore their deletion
                    if c.is_unit(&assignment) {
                        debug!("skipping unit clause deletion for ({})", c);
                    } else {
                        self.clause_db.del_clause(c);
                        debug!("deleted clause ({})", c);
                    }
                }
                Lemma::Addition(c) => {
                    debug!("checking lemma ({})", c);
                    // check if we encountered the empty clause
                    if c.is_empty() {
                        empty_clause = true;
                    }

                    // check if the lemma being added is redundant by first checking
                    // whether it is RUP, and if that doesn't work check if it is
                    // RAT
                    if check_rup(&self.clause_db, c) {
                        debug!("lemma is RUP, extending clause database");
                        self.clause_db.activate_clause(c);
                    } else if check_rat(&self.clause_db, c) {
                        debug!("lemma is RAT, extending clause database");
                        self.clause_db.activate_clause(c);
                    } else {
                        debug!("lemma is neither RUP nor RAT, refuting proof");
                        return Verdict::RefutationRefuted;
                    }

                    propagate(&self.clause_db, &mut assignment);
                }
            }

            processed += 1;
            if processed % log_cutoff == 0 {
                info!("processed {} out of {} lemmas", processed, max_lemmas);
            }
        }

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
}

/// Apply unit propagation and update the assignment correspondingly.
fn propagate(clause_db: &ClauseStorage, assignment: &mut Assignment) {
    debug!("applying unit propagation");
    trace!("before ({})", assignment);

    let mut modified = true;
    while modified {
        modified = false;
        for clause in clause_db.clauses() {
            trace!("clause {}", clause);
            if let Evaluation::Unit(lit) = clause.eval(&assignment) {
                trace!("  unit");
                assignment.add_literal(lit);
                modified = true;
            }
        }
    }

    trace!("after  ({})", assignment);
}

/// The end result of checking a proof against a formula.
pub enum Verdict {
    /// The refutation has successfully been validated.
    RefutationVerified,
    /// The refutation could not be validated as the clauses are not redundant.
    RefutationRefuted,
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
        for c in clause_db.clauses() {
            match c.eval(&assignment) {
                // if any clause evals to false then RUP is verified
                Evaluation::False => {
                    trace!("clause ({}) evaluates to false", c);
                    return true;
                }
                // if a unit is found, extend the assignment
                Evaluation::Unit(lit) => {
                    trace!("adding unit ({}) from clause ({})", lit, c);
                    assignment.add_literal(lit);
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
            trace!(
                "RUP verification failed with final assignment ({})",
                assignment
            );
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
    for clause in clause_db.clauses().filter(|c| c.has_literal(!lit)) {
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
