mod mutating_propagator;
mod propagator;

use crate::{Flags, Validator};
use anyhow::{anyhow, Result};
use indicatif::ProgressBar;
use itertools::Itertools;

use crate::common::{
    storage::{Clause, ClauseStorage, View},
    Assignment, Lemma,
};
use mutating_propagator::MutatingPropagator;
use propagator::Propagator;

pub struct MutatingChecker {
    flags: Flags,
}

impl MutatingChecker {
    fn has_rup(
        clause_db: &mut ClauseStorage,
        db_view: &View,
        propagator: &mut MutatingPropagator,
        assignment: &mut Assignment,
        lemma: Clause,
    ) -> bool {
        let rollback = assignment.rollback_point();
        for &lit in clause_db.clause(lemma) {
            if let Err(_) = assignment.try_assign(-lit) {
                assignment.rollback(rollback);
                return true;
            }
        }

        let res = propagator.propagate(clause_db, db_view, assignment);
        assignment.rollback(rollback);
        res.is_err()
    }
}

impl Validator for MutatingChecker {
    fn with_flags(flags: Flags) -> Self {
        MutatingChecker { flags }
    }

    fn validate(
        self,
        mut clause_db: ClauseStorage,
        mut db_view: View,
        proof: Vec<Lemma>,
    ) -> Result<()> {
        let mut propagator = MutatingPropagator::new(&clause_db, &db_view);
        let mut assignment = Assignment::new(&clause_db);
        propagator
            .propagate_true_units(&clause_db, &db_view, &mut assignment)
            .map_err(|_| anyhow!("prepropagation conflict"))?;

        let progress = if self.flags.progress {
            ProgressBar::new(proof.len() as u64)
        } else {
            ProgressBar::hidden()
        };
        for lemma in proof {
            match lemma {
                Lemma::Del(clause) => {
                    db_view.del(clause);
                }
                Lemma::Add(clause) => {
                    if MutatingChecker::has_rup(
                        &mut clause_db,
                        &db_view,
                        &mut propagator,
                        &mut assignment,
                        clause,
                    ) {
                        db_view.add(clause);
                        if clause_db.is_empty(clause) {
                            return Ok(());
                        }
                        if let Some(unit) = clause_db.extract_true_unit(clause) {
                            tracing::trace!("found unit in proof: {}", unit);
                            if let Err(_) = assignment.try_assign(unit) {
                                // found an early conflict
                                return Err(anyhow!("early conflict detected on literal {}", unit));
                            }
                        } else {
                            // if we found a non unit clause (more than two literals) add it to the
                            // propagator
                            propagator.add_clause(clause, &clause_db);
                        }
                        tracing::debug!("OK ({:?})", clause);
                    } else {
                        return Err(anyhow!(
                            "lemma ({}) does not have RUP ({:?})",
                            clause_db
                                .clause(clause)
                                .into_iter()
                                .map(|lit| lit.to_string())
                                .join(","),
                            clause
                        ));
                    }

                    // propagate after a clause has been added
                    if !assignment.is_empty() {
                        let _ = propagator.propagate(&mut clause_db, &db_view, &mut assignment);
                    }
                }
            }

            progress.inc(1);
        }

        Err(anyhow!("no conflict detected"))
    }
}

pub struct Checker {
    flags: Flags,
}

impl Checker {
    fn has_rup(
        clause_db: &ClauseStorage,
        db_view: &View,
        propagator: &mut Propagator,
        assignment: &mut Assignment,
        lemma: Clause,
    ) -> bool {
        let rollback = assignment.rollback_point();
        for &lit in clause_db.clause(lemma) {
            if let Err(_) = assignment.try_assign(-lit) {
                assignment.rollback(rollback);
                return true;
            }
        }

        let res = propagator.propagate(clause_db, db_view, assignment);
        assignment.rollback(rollback);
        res.is_err()
    }
}

impl Validator for Checker {
    fn with_flags(flags: Flags) -> Self {
        Checker { flags }
    }

    fn validate(
        self,
        clause_db: ClauseStorage,
        mut db_view: View,
        proof: Vec<Lemma>,
    ) -> Result<()> {
        let mut propagator = Propagator::new(&clause_db, &db_view);
        let mut assignment = Assignment::new(&clause_db);
        propagator
            .propagate_true_units(&clause_db, &db_view, &mut assignment)
            .map_err(|_| anyhow!("prepropagation conflict"))?;

        let progress = if self.flags.progress {
            ProgressBar::new(proof.len() as u64)
        } else {
            ProgressBar::hidden()
        };

        for lemma in proof {
            match lemma {
                Lemma::Del(clause) => {
                    db_view.del(clause);
                }
                Lemma::Add(clause) => {
                    if Checker::has_rup(
                        &clause_db,
                        &db_view,
                        &mut propagator,
                        &mut assignment,
                        clause,
                    ) {
                        db_view.add(clause);
                        if clause_db.is_empty(clause) {
                            return Ok(());
                        }
                        if let Some(unit) = clause_db.extract_true_unit(clause) {
                            tracing::trace!("found unit in proof: {}", unit);
                            if let Err(_) = assignment.try_assign(unit) {
                                // found an early conflict
                                return Err(anyhow!("early conflict detected on literal {}", unit));
                            }
                        } else {
                            // if we found a non unit clause (more than two literals) add it to the
                            // propagator
                            propagator.add_clause(clause);
                        }
                        tracing::debug!("OK ({:?})", clause);
                    } else {
                        return Err(anyhow!(
                            "lemma ({}) does not have RUP ({:?})",
                            clause_db
                                .clause(clause)
                                .into_iter()
                                .map(|lit| lit.to_string())
                                .join(","),
                            clause
                        ));
                    }

                    // propagate after a clause has been added
                    if !assignment.is_empty() {
                        let _ = propagator.propagate(&clause_db, &db_view, &mut assignment);
                    }
                }
            }

            progress.inc(1);
        }

        Err(anyhow!("no conflict detected"))
    }
}
