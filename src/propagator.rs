use fxhash::{FxHashMap, FxHashSet};

use itertools::Itertools;

use crate::common::{
    storage::{Clause, ClauseStorage, View},
    Assignment, Conflict, Literal,
};

pub struct Propagator<'a> {
    // TODO if we do not care about memory the hashset here could be replaced with a bitvector
    // though this might be very expensive. run benchmarks for this.
    // If the maximum value of a literal is known we can also use an array for this instead of a
    // hashset
    /// Mapping from literal to a set of clauses. These are the clauses watched by the specified
    /// literal.
    watchlist: FxHashMap<Literal, FxHashSet<Clause>>,
    /// Vec that contains the actual literal instance of the clause being watched.
    watched_by: Vec<Option<(Literal, Literal)>>,
    /// Reference to the underlying clause database to get information about the clauses.
    clause_db: &'a ClauseStorage,
}

impl<'a> Propagator<'a> {
    /// Create a new propagator from the clauses in a database.
    pub fn new(clause_db: &'a ClauseStorage) -> Self {
        // This goes through all the clauses in the database. If the clause has at least two
        // literals, the first two are registered in the watchlist. Otherwise, None is stored
        // instead, indicating that this is either a unit or empty clause.
        let mut propagator = Propagator {
            watchlist: FxHashMap::default(),
            watched_by: (0..clause_db.number_of_clauses())
                .map(|_| None)
                .collect_vec(),
            clause_db,
        };

        clause_db.all_clauses().for_each(|c| {
            let mut lits = clause_db.clause(c);
            if let (Some(fst), Some(snd)) = (lits.next(), lits.next()) {
                propagator.watch(fst, c);
                propagator.watch(snd, c);
                propagator.watched_by[c.index] = Some((fst, snd));
            }
        });

        propagator
    }
}

impl Propagator<'_> {
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
            let mut literals = db_view.clause(c);
            if let (Some(first), None) = (literals.next(), literals.next()) {
                if let e @ Err(_) = assignment.try_assign(first) {
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

        // create a copy of the literals that need to be checked
        let mut processed = 0;
        let mut result = Ok(());

        loop {
            debug_assert!(self.sanity_check());
            // return the result once we have processed everything or a conflict has been
            // encountered
            if assignment.len() <= processed || result.is_err() {
                return result;
            }

            let lit = assignment.nth_lit(processed);
            processed += 1;

            let mut fuse = false;
            let mut relevant_clauses = self.take_watched_clauses(lit);
            let new_watchlist = relevant_clauses
                .drain()
                .filter(|&clause| {
                    let (fst, snd) = self.watched_by(clause).expect("watchlist not sane");

                    if fuse
                        || !db_view.is_active(clause)
                        || assignment.is_true(fst)
                        || assignment.is_true(snd)
                    {
                        // keep if nothing happened
                        return true;
                    }
                    if let Some(new_unit) = self.update_watchlist(clause, lit, assignment) {
                        if let e @ Err(_) = assignment.try_assign(new_unit) {
                            result = e;
                            fuse = true;
                            assignment.rollback_to(rollback);
                        }
                        // if we got a new unit that means the watchlist was not mutated
                        true
                    } else {
                        false
                    }
                })
                .collect_vec();

            self.watchlist
                .entry(lit.abs())
                .or_default()
                .extend(new_watchlist);
        }
    }

    pub fn sanity_check(&self) -> bool {
        for clause in self.clause_db.all_clauses() {
            if let Some((fst, snd)) = self.watched_by(clause) {
                assert!(
                    self.watched_clauses(fst)
                        .expect("sanity check failed")
                        .contains(&clause),
                    "watchlist for {:?} does not contain {:?}",
                    fst,
                    clause
                );
                assert!(
                    self.watched_clauses(snd)
                        .expect("sanity check failed")
                        .contains(&clause),
                    "watchlist for {:?} does not contain {:?}",
                    snd,
                    clause
                );
            }
        }

        for (&literal, clause_set) in &self.watchlist {
            for clause in clause_set {
                let (fst, snd) = self.watched_by(*clause).expect("sanity check failed");
                assert!(fst != snd && !fst.matches(snd));
                assert!(fst.matches(literal) || snd.matches(literal));
            }
        }
        true
    }

    /// Takes a clause, literal and assignment. The function will try to find a new unassigned
    /// literal to watch instead of the given one.
    /// If there is no other literal available the function returns a new unit.
    fn update_watchlist(
        &mut self,
        clause: Clause,
        to_replace: Literal,
        assignment: &Assignment,
    ) -> Option<Literal> {
        if let Some((fst, snd)) = self.watched_by(clause) {
            let other = if fst.matches(to_replace) {
                snd
            } else if snd.matches(to_replace) {
                fst
            } else {
                panic!("called with wrong literal to replace")
            };
            if let Some(replacement) =
                find_next_unassigned(self.clause_db.clause(clause), assignment, fst, snd)
            {
                // found a new replacement
                self.unwatch(to_replace, clause);
                self.watch(replacement, clause);
                self.watched_by[clause.index] = Some((other, replacement));
                None
            } else {
                Some(other)
            }
        } else {
            unreachable!("clause in question was not watched by any literal")
        }
    }

    fn take_watched_clauses(&mut self, literal: Literal) -> FxHashSet<Clause> {
        self.watchlist
            .remove(&literal.abs())
            .unwrap_or(FxHashSet::default())
    }

    fn watched_clauses(&self, literal: Literal) -> Option<&FxHashSet<Clause>> {
        self.watchlist.get(&literal.abs())
    }

    fn watched_by(&self, clause: Clause) -> Option<(Literal, Literal)> {
        self.watched_by[clause.index]
    }

    fn watch(&mut self, literal: Literal, clause: Clause) {
        self.watchlist
            .entry(literal.abs())
            .or_default()
            .insert(clause);
    }

    fn unwatch(&mut self, literal: Literal, clause: Clause) {
        if let Some(set) = self.watchlist.get_mut(&literal.abs()) {
            set.remove(&clause);
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
