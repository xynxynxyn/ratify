use indicatif::ProgressBar;
use itertools::Itertools;
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
    fn validate(mut self, lemmas: Vec<RefLemma>) -> Verdict {
        info!("backward validating");

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

        let mut lemmas = self.preprocess(lemmas);
        debug!("preprocessed lemmas");

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
                RefLemma::Deletion(_c_ref) => {
                    // do nothing
                    ()
                }
                RefLemma::Addition(c_ref) => {
                    if !self.core_list.is_core(c_ref) {
                        // skip if the lemma is not marked as core
                        continue;
                    }

                    debug!("checking ({})", self.state.clause_db.get_any_clause(c_ref));

                    // Update the database to have the correct set of clauses
                    // TODO reset everything here properly
                    self.state.clause_db.del_clause(c_ref);

                    // check for redundancy
                    if !self.has_rup(c_ref) {
                        trace!("no RUP ({})", self.state.clause_db.get_any_clause(c_ref));
                        if self.state.features.rup_only {
                            return Verdict::RefutationRefuted;
                        } else if self.has_rat(c_ref) {
                            trace!(
                                "but has RAT ({})",
                                self.state.clause_db.get_any_clause(c_ref)
                            );
                        } else {
                            trace!(
                                "also no RAT ({})",
                                self.state.clause_db.get_any_clause(c_ref)
                            );
                            return Verdict::RefutationRefuted;
                        }
                    } else {
                        trace!("has RAT ({})", self.state.clause_db.get_any_clause(c_ref));
                    }
                }
            }
            progress.inc(1);
        }

        // log all the core clauses
        trace!(
            "core lemmas: {}",
            Itertools::intersperse(
                self.state
                    .clause_db
                    .all_clause_refs()
                    .filter_map(|c_ref| {
                        if self.core_list.is_core(c_ref) {
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
    fn preprocess(&mut self, lemmas: Vec<RefLemma>) -> Vec<RefLemma> {
        let mut processed = Vec::with_capacity(lemmas.len());
        // propagate initially
        propagate(
            &self.state.clause_db,
            &self.state.watcher,
            &mut self.state.assignment,
            None,
        );

        // activate all clauses that have been added
        for lemma in lemmas {
            match lemma {
                lemma @ RefLemma::Addition(c_ref) => {
                    self.state.clause_db.activate_clause(c_ref);
                    processed.push(lemma);
                }
                lemma @ RefLemma::Deletion(c_ref) => {
                    if self.state.features.skip_deletions {
                        continue;
                    }
                    // check if this is a unit deletion
                    if self.state.features.strict
                        || !self
                            .state
                            .clause_db
                            .get_any_clause(c_ref)
                            .is_unit(&self.state.assignment)
                    {
                        // if we are in strict mode (accept all deletions without checking) or the
                        // clause is not a unit clause, then delete the clause and add the
                        // deletions to the new lemmas
                        self.state.clause_db.del_clause(c_ref);
                        processed.push(lemma);
                    } else {
                        debug!(
                            "ignored unit deletion d({})",
                            self.state.clause_db.get_any_clause(c_ref)
                        );
                    }
                }
            }

            // propagation is only necessary if we actually delete stuff
            if !self.state.features.skip_deletions {
                // propagate after each step
                propagate(
                    &self.state.clause_db,
                    &self.state.watcher,
                    &mut self.state.assignment,
                    None,
                );
            }
        }

        processed
    }

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
