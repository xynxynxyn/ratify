use itertools::Itertools;

use crate::common::{
    storage::{Clause, ClauseArray, ClauseStorage, LiteralArray, View},
    Assignment, Conflict, Literal,
};

pub struct Propagator<'a> {
    /// Mapping from literal to a set of clauses. These are the clauses watched by the specified
    /// literal.
    watchlist: LiteralArray<Vec<Clause>>,
    /// Vec that contains the actual literal instance of the clause being watched.
    watched_by: ClauseArray<(Literal, Literal)>,
    /// Reference to the underlying clause database to get information about the clauses.
    clause_db: &'a ClauseStorage,
}

impl<'a> Propagator<'a> {
    /// Create a new propagator from the clauses in a database.
    pub fn new(clause_db: &'a ClauseStorage, view: &View) -> Self {
        // This goes through all the clauses in the database. If the clause has at least two
        // literals, the first two are registered in the watchlist. Otherwise, None is stored
        // instead, indicating that this is either a unit or empty clause.
        let mut propagator = Propagator {
            watchlist: clause_db.literal_array(),
            watched_by: clause_db.clause_array(),
            clause_db,
        };

        view.clauses().for_each(|c| propagator.add_clause(c));

        propagator
    }
}

impl Propagator<'_> {
    pub fn add_clause(&mut self, clause: Clause) {
        let mut lits = self.clause_db.clause(clause);
        if let (Some(fst), Some(snd)) = (lits.next(), lits.next()) {
            self.watchlist[fst].push(clause);
            self.watchlist[snd].push(clause);
            self.watched_by[clause] = (fst, snd);
        }
    }

    /// Scan the currently active clauses for true units (clauses only containing a single
    /// literal). Update the assignment accordingly. If a conflict is encountered an error is
    /// returned with the literal that caused the conflict.
    pub fn propagate_true_units(
        &self,
        db_view: &View,
        assignment: &mut Assignment,
    ) -> Result<(), Conflict> {
        let rollback = assignment.rollback_point();

        for c in db_view.clauses() {
            // check if there exists is no second literal
            // this is thus a true unit
            if let Some(unit) = self.clause_db.extract_true_unit(c) {
                if let e @ Err(_) = assignment.try_assign(unit) {
                    assignment.rollback_to(rollback);
                    return e;
                }
            }
        }
        Ok(())
    }

    /// Propagates the provided assignment, if this results in a conflict, returns an error
    /// indicating what caused the conflict and rolls back the assignment to its prior state.
    pub fn propagate(
        &mut self,
        db_view: &View,
        assignment: &mut Assignment,
    ) -> Result<(), Conflict> {
        let rollback = assignment.rollback_point();

        let mut processed = 0;
        let mut result = Ok(());

        loop {
            // return the result once we have processed everything or a conflict has been
            // encountered
            if assignment.len() <= processed || result.is_err() {
                if result.is_ok() {
                    tracing::debug!("no conflict found, final assignment: [{}]", assignment)
                }
                return result;
            }

            let lit = -assignment.nth_lit(processed);
            processed += 1;

            let mut relevant_clauses =
                std::mem::replace(&mut self.watchlist[lit], Vec::with_capacity(0));

            let mut i = 0;

            while i < relevant_clauses.len() {
                let clause = relevant_clauses[i];
                i += 1;

                if !db_view.is_active(clause) {
                    continue;
                }

                let (fst, snd) = self.watched_by[clause];

                if assignment.is_true(fst) || assignment.is_true(snd) {
                    continue;
                }

                let other = if fst == lit { snd } else { fst };

                // one of the two literals must be falsified
                // find out which one and replace it
                if let Some(next_unassigned) =
                    find_next_unassigned(self.clause_db.clause(clause), assignment, fst, snd)
                {
                    self.watchlist[next_unassigned].push(clause);
                    self.watched_by[clause] = (next_unassigned, other);
                    relevant_clauses.swap_remove(i - 1);
                    i -= 1;
                } else {
                    // Since we did not find another unassigned literal the other watched one must
                    // be a new unit
                    if let e @ Err(_) = assignment.try_assign(other) {
                        // the unit lead to a conflict
                        result = e;
                        assignment.rollback_to(rollback);
                        break;
                    }
                }
            }

            self.watchlist[lit] = relevant_clauses;
        }
    }
}

// Find a literal in the clause that is not falsified and not already watched.
fn find_next_unassigned(
    literals: impl Iterator<Item = Literal>,
    assignment: &Assignment,
    except1: Literal,
    except2: Literal,
) -> Option<Literal> {
    // TODO this could cause issues if the update function is called when one of the literals in
    // the assignment is already true
    literals
        .filter(|&lit| lit != except1 && lit != except2 && !assignment.is_true(-lit))
        .next()
}
