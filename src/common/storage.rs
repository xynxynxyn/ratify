use std::{collections::BTreeSet, ops::Range};

use fxhash::FxHashMap;

use super::Literal;

/// A clause identified by its index in a database
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Clause {
    // TODO this should not be public, create a custom array struct instead that is used to index
    pub index: usize,
}

/// Keeps track of the clauses which are currently active and has a reference to the underlying
/// database.
/// Generate a view from the database and then access the clauses through it.
pub struct View<'a> {
    active: Vec<bool>,
    db: &'a ClauseStorage,
}

impl View<'_> {
    pub fn del(&mut self, clause: Clause) {
        self.active[clause.index] = false;
    }

    pub fn add(&mut self, clause: Clause) {
        self.active[clause.index] = true;
    }

    pub fn is_active(&self, clause: Clause) -> bool {
        self.active[clause.index]
    }

    pub fn clauses(&self) -> impl Iterator<Item = Clause> + '_ {
        (0..self.db.number_of_clauses()).filter_map(|i| {
            if self.active[i] {
                Some(Clause { index: i })
            } else {
                None
            }
        })
    }

    pub fn clause(&self, clause: Clause) -> impl Iterator<Item = Literal> + '_ {
        self.db.clause(clause)
    }
}

/// The clause database stores all clauses that exist within the proof and formula.
#[derive(Debug, Default)]
pub struct ClauseStorage {
    literals: Vec<Literal>,
    ranges: Vec<Range<usize>>,
}

impl ClauseStorage {
    // how many clauses are in the database?
    pub fn number_of_clauses(&self) -> usize {
        self.ranges.len()
    }

    /// Add a new clause to the database containing the specified literals.
    pub fn add_clause(&mut self, literals: impl Iterator<Item = Literal>) -> Clause {
        let index = self.ranges.len();
        let start = self.literals.len();
        self.literals.extend(literals);
        let end = self.literals.len();
        self.ranges.push(start..end);
        Clause { index }
    }

    /// Get the literals of a clause
    pub fn clause(&self, clause: Clause) -> impl Iterator<Item = Literal> + '_ {
        self.ranges
            .get(clause.index)
            .map(|range| (self.literals[range.start..range.end]).iter().cloned())
            .expect("clause index out of bounds")
    }

    pub fn all_clauses(&self) -> impl Iterator<Item = Clause> + '_ {
        (0..self.number_of_clauses()).map(|i| Clause { index: i })
    }

    // marks the first n clauses as active
    pub fn partial_view(&self, n: usize) -> View {
        let mut active = vec![true; n];
        active.extend_from_slice(&vec![false; self.ranges.len() - n]);
        View { db: &self, active }
    }
}

#[derive(Default)]
pub struct Builder {
    clauses: FxHashMap<BTreeSet<Literal>, Clause>,
    clause_db: ClauseStorage,
}

impl Builder {
    pub fn add_clause(&mut self, clause: BTreeSet<Literal>) -> Clause {
        if let Some(&c_ref) = self.clauses.get(&clause) {
            c_ref
        } else {
            let c_ref = self.clause_db.add_clause(clause.iter().cloned());
            self.clauses.insert(clause, c_ref);
            c_ref
        }
    }

    pub fn get_clause(&self, clause: BTreeSet<Literal>) -> Clause {
        *self.clauses.get(&clause).expect("clause not known")
    }

    pub fn finish(self) -> ClauseStorage {
        self.clause_db
    }
}
