mod forward;

use anyhow::anyhow;
pub use forward::*;

use crate::{core::Symbol, watcher::Watcher, Features};
use itertools::Itertools;
use log::{debug, info, log_enabled, trace};
use std::fmt::Display;

use crate::core::{Assignment, ClauseStorage, MaybeConflict, RefLemma};

struct State {
    /// Storage for all clauses
    clause_db: ClauseStorage,
    /// Watcher keeps track of watched literal responsibilities and
    /// functioniality
    watcher: Watcher,
    /// The current assignment
    assignment: Assignment,
    /// List of features enabled
    features: Features,
}

impl State {
    fn init(clause_db: ClauseStorage, features: Features) -> anyhow::Result<Self> {
        info!("populating watchlist and watchtracker");
        let watcher = Watcher::new(&clause_db);

        info!("assigning units from initial formula");
        let mut assignment = Assignment::new();
        for lit in clause_db.clauses().filter_map(|(_, clause)| clause.unit()) {
            if assignment.conflicts(lit) {
                return Err(anyhow!(
                    "propagation yields early conflict on literal {}",
                    lit
                ));
            }
            assignment.assign(lit);
        }

        if let MaybeConflict::Conflict = propagate(&clause_db, &watcher, &mut assignment) {
            return Err(anyhow!("prepropagation yields conflict"));
        }
        debug!("prepropagation result ({})", assignment);

        Ok(State {
            clause_db,
            watcher,
            assignment,
            features,
        })
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

/// The core validator trait, simply validates the given list of lemmas.
pub trait Validator {
    fn validate(self, lemmas: &[RefLemma]) -> Verdict;
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
