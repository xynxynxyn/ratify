use indicatif::ProgressBar;
use log::{debug, error, info, trace};

use crate::{
    core::{Clause, ClauseStorage, Literal, MaybeConflict, RefLemma},
    validator::propagate,
    Features, Verdict,
};

use super::{State, Validator};

/// Validator struct that tracks the state of the validation process.
pub struct ForwardValidator {
    state: State,
}

impl ForwardValidator {
    pub fn init(clause_db: ClauseStorage, features: Features) -> anyhow::Result<Self> {
        let state = State::init(clause_db, features)?;
        Ok(ForwardValidator { state })
    }

    /// Check if the given clause has the rup property
    fn has_rup(&self, lemma: &Clause) -> bool {
        // construct a new assignment by adding all the negated literals from
        // the lemma as units
        let mut asg = self.state.assignment.clone();
        for lit in lemma.literals() {
            if asg.conflicts(!lit) {
                trace!("inverting lemma lead to conflict on {} in ({})", lit, lemma);
                return true;
            }
            asg.assign(!lit);
        }

        // try to propagate the assignment
        match propagate(&self.state.clause_db, &self.state.watcher, &mut asg) {
            MaybeConflict::Conflict => true,
            MaybeConflict::NoConflict => false,
        }
    }

    fn has_rat(&self, lemma: &Clause) -> bool {
        trace!("checking RAT property for lemma '{}'", lemma);
        // check RAT property for each pivot literal
        for lit in lemma.literals() {
            if self.check_rat_on(lemma, *lit) {
                return true;
            }
        }

        trace!("RAT verification failed");
        false
    }

    fn check_rat_on(&self, lemma: &Clause, lit: Literal) -> bool {
        trace!(
            "checking RAT property for lemma ({}) on literal ({})",
            lemma,
            lit
        );
        // find all clauses in the database which contain the
        // negation of the literal.
        for (_, clause) in self
            .state
            .clause_db
            .clauses()
            .filter(|(_, c)| c.has_literal(!lit))
        {
            // create the resolvent
            let resolvent = lemma.resolve(clause, lit);
            // check for rup property
            if !self.has_rup(&resolvent) {
                trace!("RAT verification on ({}) failed", lit);
                return false;
            }
        }
        // if all of the resolvents are RUP the lemma is RAT
        true
    }
}

impl Validator for ForwardValidator {
    /// Sequentially validate each lemma by checking if it has the RUP property.
    /// Clauses are added and removed from the clause database during this process.
    fn validate(mut self, lemmas: &[RefLemma]) -> Verdict {
        info!("forward validating only");

        let bar = if self.state.features.progress {
            ProgressBar::new(lemmas.len() as u64)
        } else {
            ProgressBar::hidden()
        };

        for lemma in lemmas {
            // verify each lemma in order
            match lemma {
                RefLemma::Deletion(c_ref) => {
                    // TODO find out how to properly identify unit clauses and
                    // ignore their deletion
                    if let Some(clause) = self.state.clause_db.get_clause(*c_ref) {
                        if clause.is_unit(&self.state.assignment) {
                            debug!("is unit clause, skipping deletion ({})", clause);
                        } else {
                            trace!("delete ({})", clause);
                            self.state.clause_db.del_clause(*c_ref);
                        }
                    } else {
                        error!(
                            "tried delete but did not exist ({})",
                            self.state.clause_db.get_any_clause(*c_ref)
                        );
                    }
                }
                RefLemma::Addition(c_ref) => {
                    let clause = self.state.clause_db.get_any_clause(*c_ref);
                    debug!("checking ({})", clause);

                    // check if the lemma being added is redundant by first checking
                    // whether it is RUP, and if that doesn't work check if it is
                    // RAT
                    if self.has_rup(clause)
                        || (!self.state.features.rup_only && self.has_rat(clause))
                    {
                        if clause.is_empty() {
                            debug!("verified the empty clause, refutation validated");
                            return Verdict::RefutationVerified;
                        }
                        debug!("is redundant ({})", clause);
                        if let Some(unit) = clause.unit() {
                            // if we add a unit, add it to the assignment after
                            // it is verified.
                            // this may already conflict with the assignment,
                            // but we don't check it since the lemma is RUP
                            if self.state.features.strict {
                                // in strict mode, check if this already conflicts
                                if self.state.assignment.conflicts(unit) {
                                    error!(
                                        "early refutation when adding unit ({}) from lemmas",
                                        unit
                                    );
                                    return Verdict::EarlyRefutation;
                                }
                            }
                            debug!("found unit in proof {}", unit);
                            self.state.assignment.assign(unit);
                        }

                        self.state.clause_db.activate_clause(*c_ref);
                    } else {
                        error!("lemma not redundant, proof refuted ({})", clause);
                        return Verdict::RefutationRefuted;
                    }

                    trace!("propagating from ({})", self.state.assignment);
                    propagate(
                        &self.state.clause_db,
                        &mut self.state.watcher,
                        &mut self.state.assignment,
                    );
                    trace!("propagating to ({})", self.state.assignment);
                }
            }

            bar.inc(1);
        }

        // if we have not seen the empty clause yet and it is not RUP, then the
        // proof does not show a conflict and therefore there is no refutation to
        // verify
        error!("no conflict detected or empty clause present");
        Verdict::NoConflict
    }
}
