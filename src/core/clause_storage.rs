use bimap::BiMap;
use std::collections::HashMap;

use itertools::Itertools;

use super::Clause;

/// A reference to a clause. We use this instead of normal references to avoid
/// issues with the borrow checker. This only works because we never actually
/// delete allocations from the clause storage and any clause reference is never
/// really invalidated.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ClauseRef(usize);

/// The purpose of this data structure is to efficiently store clauses, which
/// are a collection of literals. A variety of methods to easily and quickly
/// find relevant clauses should be provided.
#[derive(Debug)]
pub struct ClauseStorage {
    mapping: BiMap<Clause, ClauseRef>,
    // TODO make this a vec to index at some point
    active: HashMap<ClauseRef, bool>,
}

impl ClauseStorage {
    /// Create a new clause storage with a certain capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        ClauseStorage {
            // TODO maybe change this to a static vec at some point and index
            // directly instead of hashing
            mapping: BiMap::with_capacity(capacity),
            active: HashMap::with_capacity(capacity),
        }
    }

    /// Retrieve the clause associated with the reference. If the clause is not
    /// currently active as it has been deleted None is returned.
    pub fn get_clause(&self, clause_ref: ClauseRef) -> Option<&Clause> {
        if let Some(true) = self.active.get(&clause_ref) {
            self.mapping.get_by_right(&clause_ref)
        } else {
            None
        }
    }

    /// Retrieve the clause associated with the reference. It does not matter if
    /// the clause is active or not. This should never fail, if the clause does
    /// not exist it panics.
    pub fn get_any_clause(&self, clause_ref: ClauseRef) -> &Clause {
        self.mapping
            .get_by_right(&clause_ref)
            .expect("unknown clause ref")
    }

    pub fn add_clause(&mut self, clause: Clause, active: bool) -> ClauseRef {
        if let Some(c_ref) = self.mapping.get_by_left(&clause) {
            // clause exists already, fetch c_ref and update activity if needed
            if active {
                *self.active.get_mut(c_ref).expect("unknown clause ref") = true;
            }
            *c_ref
        } else {
            // clause not already stored, add it
            let c_ref = ClauseRef(self.mapping.len());
            self.mapping.insert(clause, c_ref);
            self.active.insert(c_ref, active);
            c_ref
        }
    }

    /// Add all clauses from the given iterator and set them to be active or
    /// inactive.
    pub fn add_from_iter(&mut self, clauses: impl Iterator<Item = Clause>, active: bool) {
        clauses.for_each(|clause| {
            self.add_clause(clause, active);
        })
    }

    /// Activate the provided clause in the storage.
    pub fn activate_clause(&mut self, clause_ref: ClauseRef) {
        if let Some(a) = self.active.get_mut(&clause_ref) {
            *a = true;
        }
    }

    /// Deactivates the provided clause.
    pub fn del_clause(&mut self, clause_ref: ClauseRef) {
        if let Some(a) = self.active.get_mut(&clause_ref) {
            *a = false;
        }
    }

    pub fn clauses(&self) -> impl Iterator<Item = (ClauseRef, &Clause)> {
        self.mapping.iter().filter_map(|(clause, c_ref)| {
            if let Some(true) = self.active.get(c_ref) {
                Some((*c_ref, clause))
            } else {
                None
            }
        })
    }

    pub fn all_clause_refs(&self) -> impl Iterator<Item = ClauseRef> + '_ {
        self.mapping.iter().map(|(_, c_ref)| *c_ref)
    }

    pub fn dump(&self) -> String {
        Itertools::intersperse(
            self.mapping.iter().map(|(clause, c_ref)| {
                format!(
                    "{} | ({})",
                    self.active.get(c_ref).expect("invalid clause ref"),
                    clause
                )
            }),
            "\n".to_string(),
        )
        .collect()
    }
}
