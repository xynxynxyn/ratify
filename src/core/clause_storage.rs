use std::collections::HashMap;

use super::Clause;

/// The purpose of this data structure is to efficiently store clauses, which
/// are a collection of literals. A variety of methods to easily and quickly
/// find relevant clauses should be provided.
#[derive(Debug)]
pub struct ClauseStorage {
    clauses: HashMap<Clause, bool>,
}

impl ClauseStorage {
    /// Create a new clause storage with a certain capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        ClauseStorage {
            clauses: HashMap::with_capacity(capacity),
        }
    }

    /// Add all clauses from the given iterator and set them to be active or
    /// inactive.
    pub fn add_from_iter(&mut self, clauses: impl Iterator<Item = Clause>, active: bool) {
        self.clauses.extend(clauses.map(|c| (c, active)));
    }

    /// Activate the provided clause in the storage.
    pub fn activate_clause(&mut self, clause: &Clause) {
        self.clauses.get_mut(clause).map(|b| *b = true);
    }

    /// Deactivates the provided clause.
    pub fn del_clause(&mut self, clause: &Clause) {
        self.clauses.get_mut(clause).map(|_| false);
    }

    pub fn clauses(&self) -> impl Iterator<Item = &Clause> {
        self.clauses
            .iter()
            .filter_map(|(c, a)| if *a { Some(c) } else { None })
    }
}
