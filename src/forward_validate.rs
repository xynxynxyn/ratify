use anyhow::{anyhow, Result};
use indicatif::ProgressBar;
use itertools::Itertools;

use crate::{
    common::{
        storage::{Clause, ClauseStorage, View},
        Assignment, Lemma,
    },
    propagator::Propagator,
};

pub fn validate(clause_db: &ClauseStorage, mut db_view: View, proof: Vec<Lemma>) -> Result<()> {
    let mut propagator = Propagator::new(&clause_db);
    let mut assignment = Assignment::default();
    propagator
        .propagate_true_units(&db_view, &mut assignment)
        .map_err(|_| anyhow!("prepropagation conflict"))?;
    debug_assert!(propagator.sanity_check());

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
                        assignment.force_assign(unit);
                    }
                    tracing::debug!("OK {:?}", clause_db.clause(clause).collect_vec());
                } else {
                    return Err(anyhow!(
                        "lemma {:?} did not have RUP",
                        clause_db.clause(clause).collect_vec()
                    ));
                }
            }
        }

        progress.inc(1);

        if let Err(_) = propagator.propagate(&db_view, &mut assignment) {
            return Ok(());
        }
        debug_assert!(propagator.sanity_check());
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
            return true;
        }
    }

    let res = propagator.propagate(db_view, assignment);
    assignment.rollback_to(rollback);
    res.is_err()
}
