use std::collections::BTreeSet;

use indicatif::ProgressBar;
use itertools::{Either, Itertools};
use log::{debug, info, trace};

use crate::{
    core::{Assignment, ClauseRef, ClauseStorage, MaybeConflict, RefLemma},
    Features, Verdict,
};

use super::{propagate, State, Validator};

pub struct CoreList {
    inner: Vec<bool>,
}

impl CoreList {
    fn new(capacity: usize) -> Self {
        CoreList {
            inner: vec![false; capacity],
        }
    }

    pub fn mark_core(
        &mut self,
        resolvent: ClauseRef,
        mut unit_stack: Vec<ClauseRef>,
        clause_db: &ClauseStorage,
    ) {
        self.mark(resolvent);
        trace!("marked ({})", clause_db.get_any_clause(resolvent));
        let mut resolvent = clause_db.get_any_clause(resolvent).clone();

        while let Some(c_ref) = unit_stack.pop() {
            let other = clause_db.get_any_clause(c_ref);
            let lit = resolvent
                .literals()
                .filter(|lit| other.has_literal(!**lit))
                .next();
            if let Some(lit) = lit {
                self.mark(c_ref);
                trace!("marked ({})", clause_db.get_any_clause(c_ref));
                resolvent = resolvent.resolve(other, *lit);
                trace!("new resolvent ({})", resolvent);
            }
        }
    }

    fn mark(&mut self, c_ref: ClauseRef) {
        self.inner[c_ref.to_index()] = true;
    }

    fn is_core(&self, c_ref: ClauseRef) -> bool {
        self.inner[c_ref.to_index()]
    }
}

/// Validator struct that tracks the state of the validation process.
pub struct BackwardValidator {
    state: State,
    core_list: CoreList,
}

impl Validator for BackwardValidator {
    fn validate(mut self, mut lemmas: Vec<RefLemma>) -> Verdict {
        info!("backward validating");
        // get the add and delete clauses
        let (mut add, mut delete): (BTreeSet<ClauseRef>, BTreeSet<ClauseRef>) =
            lemmas.iter().partition_map(|lemma| match lemma {
                RefLemma::Addition(c_ref) => Either::Left(c_ref),
                RefLemma::Deletion(c_ref) => Either::Right(c_ref),
            });
        // all the formula clauses
        let default = self
            .state
            .clause_db
            .clauses()
            .map(|(c_ref, _)| c_ref)
            .collect_vec();
        debug!("separated addition and deletion lemmas");

        // get the empty clause or return if its not in the clause storage
        let empty_clause = match self
            .state
            .clause_db
            .all_clause_refs()
            .filter(|c_ref| self.state.clause_db.get_any_clause(*c_ref).is_empty())
            .next()
        {
            Some(e) => e,
            None => return Verdict::NoConflict,
        };

        debug!("found empty clause");
        // mark the empty clause as core
        self.core_list.mark(empty_clause);

        let progress = if self.state.features.progress {
            ProgressBar::new(lemmas.len() as u64)
        } else {
            ProgressBar::hidden()
        };

        // TODO before we start validating one has to go through all the lemmas forward and
        // activate/deactivate clauses accordingly to simulate the correct state at the end.
        // See Fig 3.10 on page 69
        while let Some(lemma) = lemmas.pop() {
            // TODO the formula has to be modified here depending on whether a lemma is removed or
            // added in reverse
            match lemma {
                RefLemma::Addition(c_ref) => add.remove(&c_ref),
                RefLemma::Deletion(c_ref) => {
                    delete.remove(&c_ref);
                    continue;
                }
            };

            let c_ref = lemma.into_c_ref();
            if !self.core_list.is_core(c_ref) {
                // skip if the lemma is not marked as core
                continue;
            }

            debug!("checking ({})", self.state.clause_db.get_any_clause(c_ref));

            // Update the database to have the correct set of clauses
            // TODO reset everything here properly
            self.state.clause_db.clean();
            for c_ref in &default {
                self.state.clause_db.activate_clause(*c_ref);
            }
            for c_ref in &add {
                self.state.clause_db.activate_clause(*c_ref);
            }
            for c_ref in &delete {
                self.state.clause_db.del_clause(*c_ref);
            }

            progress.inc(1);

            // check for redundancy
            if !self.has_rup(c_ref) {
                trace!("no RUP ({})", self.state.clause_db.get_any_clause(c_ref));
                if self.state.features.rup_only {
                    return Verdict::RefutationRefuted;
                } else if self.has_rat(c_ref) {
                    trace!("has RAT ({})", self.state.clause_db.get_any_clause(c_ref));
                    continue;
                } else {
                    trace!("no RAT ({})", self.state.clause_db.get_any_clause(c_ref));
                    return Verdict::RefutationRefuted;
                }
            } else {
                trace!("has RAT ({})", self.state.clause_db.get_any_clause(c_ref));
                continue;
            }
        }

        // log all the core clauses
        trace!(
            "core lemmas: {}",
            Itertools::intersperse(
                self.state
                    .clause_db
                    .all_clause_refs()
                    .filter_map(|c_ref| {
                        if self.core_list.is_core(c_ref) && !default.contains(&c_ref) {
                            Some(format!("({})", self.state.clause_db.get_any_clause(c_ref)))
                        } else {
                            None
                        }
                    })
                    .sorted(),
                ", ".to_string()
            )
            .collect::<String>()
        );

        return Verdict::RefutationVerified;
    }
}

impl BackwardValidator {
    pub fn init(clause_db: ClauseStorage, features: Features) -> anyhow::Result<Self> {
        let core_list = CoreList::new(clause_db.size());
        let state = State::init(clause_db, features)?;
        Ok(BackwardValidator { state, core_list })
    }

    fn has_rup(&mut self, lemma: ClauseRef) -> bool {
        let mut assignment = Assignment::from(self.state.clause_db.get_any_clause(lemma));

        match propagate(
            &self.state.clause_db,
            &self.state.watcher,
            &mut assignment,
            Some(&mut self.core_list),
        ) {
            MaybeConflict::Conflict => true,
            MaybeConflict::NoConflict => false,
        }
    }

    fn has_rat(&mut self, _lemma: ClauseRef) -> bool {
        todo!();
    }
}
