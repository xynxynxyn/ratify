use std::cell::RefCell;
use std::collections::HashMap;

use itertools::Itertools;
use log::{info, trace};

use crate::core::{Assignment, ClauseRef, ClauseStorage, Literal};

pub struct Watcher {
    /// A mapping from literals to clauses, keeping track of which literals are
    /// associated with a clause. The invariant here is that every clause should
    /// be in the watchlist of exactly two distinct literals.
    /// Literals are unnegated upon entry and when retrieving the set of clauses
    /// for a given literal as we want to consider all clauses associated with a
    /// literal, no matter the negation of said literal.
    /// Therefore, another invariant is that each key literal in the map must be
    /// unnegated.
    watchlist: RefCell<HashMap<Literal, Vec<ClauseRef>>>,
    /// A secondary map keeping track of the relationship from the other side.
    watchtracker: RefCell<HashMap<ClauseRef, (Literal, Literal)>>,
    /// Empty placeholder Vec for returning an empty iterator in the case that
    /// there is no literal present in the watchlist.
    /// Kinda hacky, maybe find another workaround.
    empty: Vec<ClauseRef>,
}

impl Watcher {
    /// Initialize the watchlist based on a given clause storage. This will
    /// assume no prior assignment and assigns the first two literals in each
    /// clause to be the watched literals respectively.
    pub fn init(clause_db: &ClauseStorage) -> Self {
        info!("populating watchlist and watchtracker");
        let mut watchlist: HashMap<Literal, Vec<ClauseRef>> = HashMap::new();
        let mut watchtracker = HashMap::new();
        for c_ref in clause_db.all_clause_refs() {
            let clause = clause_db.get_any_clause(c_ref);
            // if the clause is trivial (unit or empty), skip
            if clause.is_trivial() {
                continue;
            }

            // each clause after this must have at least 2 literals since
            // otherwise it would be trivial
            let first_two = clause.literals().take(2).collect_vec();
            if first_two.len() != 2 {
                // just in case check anyways
                continue;
            }

            watchlist.entry(first_two[0].abs()).or_default().push(c_ref);
            watchlist.entry(first_two[1].abs()).or_default().push(c_ref);
            watchtracker.insert(c_ref, (*first_two[0], *first_two[1]));
        }
        Watcher {
            watchlist: RefCell::new(watchlist),
            watchtracker: RefCell::new(watchtracker),
            empty: vec![],
        }
    }

    /// Retrieve the set of clause references that are watching a given literal.
    pub fn watched_by(&self, literal: Literal) -> Vec<ClauseRef> {
        self.watchlist
            .borrow()
            .get(&literal.abs())
            .unwrap_or_else(|| &self.empty)
            .to_vec()
    }

    /// Return the two literals that a clause is watching.
    pub fn watches(&self, c_ref: ClauseRef) -> Option<(Literal, Literal)> {
        self.watchtracker.borrow().get(&c_ref).copied()
    }

    /// Unwatches the given literal from the clause and watches a new unassigned
    /// literal. If the clause ends up evaluating to unit (there is no other
    /// unassigned literal), then this function returns that literal.
    pub fn unwatch_and_watch(
        &self,
        c_ref: ClauseRef,
        literal: Literal,
        clause_db: &ClauseStorage,
        assignment: &Assignment,
    ) -> Option<Literal> {
        // get the clause and the literals it watches first
        let (lit1, lit2) = self.watches(c_ref)?;
        let clause = clause_db.get_any_clause(c_ref);
        trace!("unwatching lit {} for clause {}", literal, clause);

        let mut watchlist = self.watchlist.borrow_mut();
        let mut watchtracker = self.watchtracker.borrow_mut();

        // go through all the literals in the clause to find the next unassigned
        // literal we can watch.
        if let Some(next_unassigned) = clause
            .literals()
            .filter(|&lit| {
                // ignore all the literals which are falsified in the assignment
                // or which are already watched.
                !(assignment.has_literal(&!lit) || *lit == lit1 || *lit == lit2)
            })
            .next()
        {
            trace!("found next unassigned lit {}", next_unassigned);
            // found an unassigned literal to switch to
            // remove the clause from the watchlist and then swap to the new one
            if let Some(w_list) = watchlist.get_mut(&literal.abs()) {
                if let Some(index) = w_list.iter().position(|c| c_ref == *c) {
                    w_list.swap_remove(index);
                    trace!("removed clause from watchlist with index {}", index);

                    // assign the clause to the new unassigned literal in the
                    // watchlist
                    watchlist
                        .entry(next_unassigned.abs())
                        .or_default()
                        .push(c_ref);
                    trace!(
                        "added clause to watchlist for lit {}",
                        next_unassigned.abs()
                    );

                    // update the watchtracker too
                    if let Some(watches) = watchtracker.get_mut(&c_ref) {
                        if literal.equal(&watches.0) {
                            // unwatch the literal
                            *watches = (*next_unassigned, watches.1);
                        } else {
                            *watches = (watches.0, *next_unassigned);
                        }
                        trace!("updated watchtracker to ({}, {})", watches.0, watches.1);
                    }
                }
            }
            // return None since we did not encounter a unit
            None
        } else {
            trace!("no next unassigned, must be unit");
            // no unassigned literal found, this must be a unit clause
            if literal.equal(&lit1) {
                // lit1 is the known literal, lit2 must be the unit
                Some(lit2)
            } else {
                // lit2 is the known literal, lit1 must be the unit
                Some(lit1)
            }
        }
    }
}
