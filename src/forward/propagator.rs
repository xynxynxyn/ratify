use crate::common::{
    storage::{Clause, ClauseStorage, View},
    Assignment, Conflict,
};

mod immutable;
mod mutating;
mod naive;

pub use immutable::*;
pub use mutating::*;
pub use naive::*;

pub trait Propagator {
    fn init(clause_db: &ClauseStorage, db_view: &View) -> Self;

    fn propagate(
        &mut self,
        clause_db: &mut ClauseStorage,
        assignment: &mut Assignment,
    ) -> Result<(), Conflict>;

    fn propagate_true_units(
        &self,
        clause_db: &ClauseStorage,
        db_view: &View,
        assignment: &mut Assignment,
    ) -> Result<(), Conflict> {
        for c in clause_db.clauses(db_view) {
            // check if there exists is no second literal
            // this is thus a true unit
            if let Some(unit) = clause_db.extract_true_unit(c) {
                if let e @ Err(_) = assignment.try_assign(unit) {
                    return e.map(|_| ());
                }
            }
        }
        Ok(())
    }

    fn add_clause(&mut self, clause: Clause, clause_db: &ClauseStorage);

    fn delete_clause(&mut self, clause: Clause, clause_db: &ClauseStorage);
}
