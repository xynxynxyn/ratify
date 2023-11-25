mod propagator;

use crate::{Flags, Validator};
use anyhow::{anyhow, Result};
use indicatif::ProgressBar;

use crate::common::{
    storage::{Clause, ClauseStorage, View},
    Assignment, Lemma,
};

use propagator::*;

pub struct Checker<P> {
    flags: Flags,
    clause_db: ClauseStorage,
    db_view: View,
    propagator: P,
}

pub type NaiveChecker = Checker<NaivePropagator>;

impl Validator for NaiveChecker {
    fn init(flags: Flags, clause_db: ClauseStorage, db_view: View) -> Self {
        let propagator = NaivePropagator::init(&clause_db, &db_view);
        Checker {
            flags,
            clause_db,
            db_view,
            propagator,
        }
    }

    fn validate(self, proof: Vec<Lemma>) -> anyhow::Result<()> {
        validate(self, proof)
    }
}

pub type ConstChecker = Checker<ConstPropagator>;

impl Validator for ConstChecker {
    fn init(flags: Flags, clause_db: ClauseStorage, db_view: View) -> Self {
        let propagator = ConstPropagator::init(&clause_db, &db_view);
        Checker {
            flags,
            clause_db,
            db_view,
            propagator,
        }
    }

    fn validate(self, proof: Vec<Lemma>) -> anyhow::Result<()> {
        validate(self, proof)
    }
}

pub type MutatingChecker = Checker<MutatingPropagator>;

impl Validator for MutatingChecker {
    fn init(flags: Flags, clause_db: ClauseStorage, db_view: View) -> Self {
        let propagator = MutatingPropagator::init(&clause_db, &db_view);
        Checker {
            flags,
            clause_db,
            db_view,
            propagator,
        }
    }

    fn validate(self, proof: Vec<Lemma>) -> anyhow::Result<()> {
        validate(self, proof)
    }
}

fn validate<P: Propagator>(checker: Checker<P>, proof: Vec<Lemma>) -> Result<()> {
    let mut clause_db = checker.clause_db;
    let mut propagator = checker.propagator;
    let mut db_view = checker.db_view;
    let mut assignment = Assignment::new(&clause_db);
    propagator
        .propagate_true_units(&clause_db, &db_view, &mut assignment)
        .map_err(|_| anyhow!("assignment of true units yielded conflict"))?;
    propagator
        .propagate(&mut clause_db, &mut assignment)
        .map_err(|_| anyhow!("prepropagation yielded conflict"))?;

    let progress = if checker.flags.progress {
        ProgressBar::new(proof.len() as u64)
    } else {
        ProgressBar::hidden()
    };

    let mut step = 0;

    for lemma in proof {
        match lemma {
            Lemma::Del(clause) => {
                // check if the clause to be deleted is unit
                if clause_db.is_unit(clause, &assignment) {
                    tracing::warn!(
                        "ignoring deletion of unit clause {} {}",
                        clause,
                        clause_db.print_clause(clause)
                    );
                } else {
                    propagator.delete_clause(clause, &clause_db);
                    db_view.del(clause);
                }
            }
            Lemma::Add(clause) => {
                if has_rup(&mut clause_db, &mut propagator, &mut assignment, clause) {
                    let already_added = db_view.is_active(clause);
                    db_view.add(clause);
                    if clause_db.is_empty(clause) {
                        return Ok(());
                    }
                    if let Some(unit) = clause_db.extract_true_unit(clause) {
                        tracing::debug!("found unit in proof: {}", unit);
                        assignment
                            .try_assign(unit)
                            .map_err(|_| anyhow!("early conflict detected on literal {}", unit))?;
                    } else {
                        // if we found a non unit clause (more than two literals) add it to the
                        // propagator. do not add it again if it was already present before,
                        // this would corrupt the watchlists potentially
                        if already_added {
                        } else if assignment.is_satisfied(clause, &clause_db) {
                            tracing::warn!("clause is already satisfied, not adding to propagator");
                        } else {
                            propagator.add_clause(clause, &clause_db);
                        }
                    }

                    // propagate after a clause has been added
                    if let Err(_) = propagator.propagate(&mut clause_db, &mut assignment) {
                        tracing::warn!("early conflict detected");
                        return Ok(());
                    }

                    tracing::trace!("OK {}", clause);
                } else {
                    return Err(anyhow!(
                        "#{} lemma ({}) does not have RUP {}",
                        step,
                        clause_db.print_clause(clause),
                        clause,
                    ));
                }
            }
        }

        step += 1;
        progress.inc(1);
    }

    Err(anyhow!("no conflict detected"))
}

fn has_rup(
    clause_db: &mut ClauseStorage,
    propagator: &mut impl Propagator,
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

    let res = propagator.propagate(clause_db, assignment);
    assignment.rollback(rollback);
    res.is_err()
}
