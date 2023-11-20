use crate::common::{
    storage::{Clause, ClauseStorage, LiteralArray, View},
    Assignment, Conflict,
};

pub struct MutatingPropagator {
    watchlist: LiteralArray<Vec<Clause>>,
}

impl MutatingPropagator {
    pub fn new(clause_db: &ClauseStorage, view: &View) -> Self {
        let mut propagator = MutatingPropagator {
            watchlist: clause_db.literal_array(),
        };

        clause_db
            .clauses(view)
            .for_each(|c| propagator.add_clause(c, clause_db));

        propagator
    }

    pub fn add_clause(&mut self, clause: Clause, clause_db: &ClauseStorage) {
        let lits = clause_db.clause(clause);
        if lits.len() >= 2 {
            self.watchlist[lits[0]].push(clause);
            self.watchlist[lits[1]].push(clause);
        }
    }

    pub fn propagate_true_units(
        &self,
        clause_db: &ClauseStorage,
        db_view: &View,
        assignment: &mut Assignment,
    ) -> Result<(), Conflict> {
        let rollback = assignment.rollback_point();

        for c in clause_db.clauses(db_view) {
            // check if there exists is no second literal
            // this is thus a true unit
            if let Some(unit) = clause_db.extract_true_unit(c) {
                if let e @ Err(_) = assignment.try_assign(unit) {
                    assignment.rollback(rollback);
                    return e;
                }
            }
        }
        Ok(())
    }

    pub fn propagate(
        &mut self,
        clause_db: &mut ClauseStorage,
        assignment: &mut Assignment,
    ) -> Result<(), Conflict> {
        let mut processed = 0;

        while processed < assignment.trace_len() {
            // return the result once we have processed everything or a conflict has been
            // encountered
            let lit = -assignment.nth_lit(processed);
            processed += 1;

            let mut relevant_clauses = std::mem::replace(&mut self.watchlist[lit], vec![]);
            let mut i = 0;

            while i < relevant_clauses.len() {
                let clause = relevant_clauses[i];
                i += 1;

                let (fst, snd) = clause_db.first_two_literals(clause).unwrap();

                let (other, swap_with) = if fst == lit { (snd, 0) } else { (fst, 1) };

                if assignment.is_true(other) {
                    continue;
                }

                if let Some(next_unassigned) =
                    clause_db.next_non_falsified_and_swap(clause, assignment, swap_with)
                {
                    self.watchlist[next_unassigned].push(clause);
                    i -= 1;
                    relevant_clauses.swap_remove(i);
                } else {
                    // Since we did not find another unassigned literal the other watched one must
                    // be a new unit
                    if let e @ Err(_) = assignment.try_assign(other) {
                        // the unit lead to a conflict
                        self.watchlist[lit] = relevant_clauses;
                        return e;
                    }
                }
            }

            self.watchlist[lit] = relevant_clauses;
        }

        Ok(())
    }

    pub fn delete_clause(&mut self, clause_db: &ClauseStorage, clause: Clause) {
        let (fst, snd) = clause_db.first_two_literals(clause).unwrap();
        if !self.watchlist[fst].contains(&clause) {
            panic!("panic {} not in watchlist: {}", fst, clause);
        }
        if !self.watchlist[snd].contains(&clause) {
            panic!("panic {} not in watchlist: {}", snd, clause);
        }
        self.watchlist[fst].retain(|&c| c != clause);
        self.watchlist[snd].retain(|&c| c != clause);
        if self.watchlist[fst].contains(&clause) {
            panic!("panic {} still in watchlist", fst);
        }
        if self.watchlist[snd].contains(&clause) {
            panic!("panic {} still in watchlist", snd);
        }
    }
}
