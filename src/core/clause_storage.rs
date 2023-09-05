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
    clauses: Vec<(Clause, bool)>,
}

impl ClauseStorage {
    /// Create a new clause storage with a certain capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        ClauseStorage {
            clauses: Vec::with_capacity(capacity),
        }
    }

    /// Retrieve the clause associated with the reference. If the clause is not
    /// currently active as it has been deleted None is returned.
    pub fn get_clause(&self, clause_ref: ClauseRef) -> Option<&Clause> {
        if let Some((c, true)) = self.clauses.get(clause_ref.0) {
            Some(c)
        } else {
            None
        }
    }

    /// Retrieve the clause associated with the reference. It does not matter if
    /// the clause is active or not. This should never fail, if the clause does
    /// not exist it panics.
    pub fn get_any_clause(&self, clause_ref: ClauseRef) -> &Clause {
        if let Some((c, _)) = self.clauses.get(clause_ref.0) {
            c
        } else {
            panic!("unknown clause reference")
        }
    }

    pub fn add_clause(&mut self, clause: Clause, active: bool) -> ClauseRef {
        self.clauses.push((clause, active));
        ClauseRef(self.clauses.len() - 1)
    }

    /// Add all clauses from the given iterator and set them to be active or
    /// inactive.
    pub fn add_from_iter(&mut self, clauses: impl Iterator<Item = Clause>, active: bool) {
        self.clauses.extend(clauses.map(|c| (c, active)));
    }

    /// Activate the provided clause in the storage.
    pub fn activate_clause(&mut self, clause_ref: ClauseRef) {
        if let Some((_, b)) = self.clauses.get_mut(clause_ref.0) {
            *b = true;
        }
    }

    /// Deactivates the provided clause.
    pub fn del_clause(&mut self, clause_ref: ClauseRef) {
        self.clauses.get_mut(clause_ref.0).map(|_| false);
    }

    pub fn clauses(&self) -> impl Iterator<Item = (ClauseRef, &Clause)> {
        self.clauses
            .iter()
            .enumerate()
            .filter_map(|(i, (c, b))| if *b { Some((ClauseRef(i), c)) } else { None })
    }

    pub fn all_clause_refs(&self) -> impl Iterator<Item = ClauseRef> {
        (0..self.clauses.len()).map(|i| ClauseRef(i))
    }

    pub fn clause_refs(&self) -> impl Iterator<Item = ClauseRef> + '_ {
        self.clauses
            .iter()
            .enumerate()
            .filter_map(|(i, (_, b))| if *b { Some(ClauseRef(i)) } else { None })
    }
}
