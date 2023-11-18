mod propagator;

use anyhow::{anyhow, Result};
use indicatif::ProgressBar;
use itertools::Itertools;

use crate::common::{
    storage::{Clause, ClauseStorage, View},
    Assignment, Lemma,
};
use propagator::Propagator;

pub fn validate(clause_db: &ClauseStorage, mut db_view: View, proof: Vec<Lemma>) -> Result<()> {
    let mut propagator = Propagator::new(&clause_db, &db_view);
    let mut assignment = Assignment::new(clause_db);
    propagator
        .propagate_true_units(&db_view, &mut assignment)
        .map_err(|_| anyhow!("prepropagation conflict"))?;

    let progress = ProgressBar::new(proof.len() as u64);
    for lemma in proof {
        match lemma {
            Lemma::Del(clause) => {
                db_view.del(clause);
            }
            Lemma::Add(clause) => {
                if has_rup(&db_view, &mut propagator, &mut assignment, clause) {
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
                    let _ = propagator.propagate(&db_view, &mut assignment);
                }
            }
        }

        progress.inc(1);
    }

    Err(anyhow!("no conflict detected"))
}

fn has_rup(
    db_view: &View,
    propagator: &mut Propagator,
    assignment: &mut Assignment,
    lemma: Clause,
) -> bool {
    let rollback = assignment.rollback_point();
    for &lit in db_view.clause(lemma) {
        if let Err(_) = assignment.try_assign(-lit) {
            assignment.rollback(rollback);
            return true;
        }
    }

    let res = propagator.propagate(db_view, assignment);
    assignment.rollback(rollback);
    res.is_err()
}
