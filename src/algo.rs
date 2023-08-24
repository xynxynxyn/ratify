use std::fmt::Display;

use log::{debug, error, info};

use crate::core::{Assignment, Clause, ClauseStorage, Evaluation, Lemma};

/// The end result of checking a proof against a formula.
pub enum Validity {
    /// The refutation has successfully been validated.
    RefutationVerified,
    /// The refutation could not be validated as the clauses are not redundant.
    RefutationRefuted,
    /// The proof does not yield an empty clause. Therefore, there is no
    /// refutation to validate.
    NoConflict,
}

impl Display for Validity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Validity::RefutationVerified => write!(f, "s VERIFIED"),
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
    // create a new assignment from the lemma
    let mut assignment = Assignment::from(lemma);
    // track whether the assignment has been modified
    let mut modified = false;
    loop {
        // check each clause in the database
        for c in clause_db.clauses() {
            debug!("checking clause {} against assignment {}", c, assignment);
            match c.eval(&assignment) {
                // if any clause evals to false then RUP is verified
                Evaluation::False => return true,
                // if a unit is found, extend the assignment
                Evaluation::Unit(lit) => {
                    debug!("adding unit {}", lit);
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
            return false;
        }
    }
}

/// Sequentially validate each lemma by checking if it has the RUP property.
/// Clauses are added and removed from the clause database during this process.
pub fn forward_validate(clause_db: &mut ClauseStorage, lemmas: &[Lemma]) -> Validity {
    info!("forward validating rup only");
    let mut empty_clause = false;

    for lemma in lemmas {
        // verify each lemma in order
        match lemma {
            Lemma::Deletion(c) => {
                debug!("deleting clause '{}'", c);
                clause_db.del_clause(c)
            }
            Lemma::Addition(c) => {
                debug!("checking lemma '{}'", c);
                // check if we encountered the empty clause
                if c.is_empty() {
                    empty_clause = true;
                }

                // check if the lemma being added is redundant by first checking
                // whether it is RUP, and if that doesn't work check if it is
                // RAT
                if check_rup(clause_db, c) {
                    debug!("lemma is rup, extending clause database");
                    clause_db.add_clause(c.clone());
                } else {
                    // TODO check for RAT property as fallback
                    debug!("lemma is NOT rup, proof not valid");
                    return Validity::RefutationRefuted;
                }
            }
        }
    }

    // if we have not seen the empty clause yet and it is not RUP, then the
    // proof does not show a conflict and therefore there is no refutation to
    // verify
    if !empty_clause && !check_rup(clause_db, &Clause::empty()) {
        error!("no conflict detected");
        Validity::NoConflict
    } else {
        Validity::RefutationVerified
    }
}
