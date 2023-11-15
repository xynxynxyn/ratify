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
    let mut propagator = Propagator::new(&clause_db);
    let mut assignment = clause_db.new_assignment();
    propagator
        .propagate_true_units(&db_view, &mut assignment)
        .map_err(|_| anyhow!("prepropagation conflict"))?;
    debug_assert!(propagator.sanity_check());
    tracing::debug!("initial assignment: {}", assignment);

    let progress = ProgressBar::new(proof.len() as u64);
    for lemma in proof {
        match lemma {
            Lemma::Del(clause) => {
                db_view.del(clause);
            }
            Lemma::Add(clause) => {
                if clause == (Clause { index: 461 }) {
                    println!("check");
                }
                if has_rup(&db_view, &mut propagator, &mut assignment, clause) {
                    db_view.add(clause);
                    if clause_db.is_empty(clause) {
                        return Ok(());
                    }
                    if let Some(unit) = clause_db.extract_true_unit(clause) {
                        tracing::trace!("found unit in proof: {}", unit);
                        assignment.force_assign(unit);
                    }
                    tracing::debug!("OK ({:?})", clause);
                } else {
                    return Err(anyhow!(
                        "lemma ({}) does not have RUP ({:?})",
                        clause_db
                            .clause(clause)
                            .map(|lit| lit.to_string())
                            .join(","),
                        clause
                    ));
                }
            }
        }

        progress.inc(1);

        let _ = propagator.propagate(&db_view, &mut assignment);
        //debug_assert!(propagator.sanity_check());
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
    for lit in db_view.clause(lemma) {
        if let Err(_) = assignment.try_assign(-lit) {
            assignment.rollback_to(rollback);
            return true;
        }
    }

    let res = propagator.propagate(db_view, assignment);
    assignment.rollback_to(rollback);
    res.is_err()
}
