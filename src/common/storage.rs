use std::{
    collections::BTreeSet,
    ops::{Index, IndexMut, Range},
};

use fxhash::FxHashMap;

use super::{Assignment, Literal};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralArray<T> {
    inner: Vec<T>,
    max_literal: i32,
}

impl<T> Index<Literal> for LiteralArray<T> {
    type Output = T;
    #[inline]
    fn index(&self, index: Literal) -> &Self::Output {
        let index = index.raw();
        if index < 0 {
            unsafe {
                self.inner
                    .get_unchecked((-index + self.max_literal) as usize)
            }
        } else {
            unsafe { self.inner.get_unchecked(index as usize) }
        }
    }
}

impl<T> IndexMut<Literal> for LiteralArray<T> {
    #[inline]
    fn index_mut(&mut self, index: Literal) -> &mut Self::Output {
        let index = index.raw();
        if index < 0 {
            unsafe {
                self.inner
                    .get_unchecked_mut((-index + self.max_literal) as usize)
            }
        } else {
            unsafe { self.inner.get_unchecked_mut(index as usize) }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LiteralSet {
    pub(super) inner: LiteralArray<bool>,
}

impl LiteralSet {
    pub fn insert(&mut self, lit: Literal) -> bool {
        let already_present = self.contains(lit);
        self.inner[lit] = true;
        !already_present
    }

    pub fn contains(&self, lit: Literal) -> bool {
        self.inner[lit]
    }

    pub fn remove(&mut self, lit: Literal) -> bool {
        let already_present = self.contains(lit);
        self.inner[lit] = false;
        already_present
    }
}

/// A clause identified by its index in a database
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Clause {
    // TODO this should not be public, create a custom array struct instead that is used to index
    pub index: usize,
}

pub struct ClauseArray<T> {
    inner: Vec<T>,
}

impl<T> Index<Clause> for ClauseArray<T> {
    type Output = T;
    fn index(&self, c: Clause) -> &Self::Output {
        unsafe { self.inner.get_unchecked(c.index) }
    }
}

impl<T> IndexMut<Clause> for ClauseArray<T> {
    fn index_mut(&mut self, c: Clause) -> &mut Self::Output {
        unsafe { self.inner.get_unchecked_mut(c.index) }
    }
}

/// Keeps track of the clauses which are currently active and has a reference to the underlying
/// database.
/// Generate a view from the database and then access the clauses through it.
pub struct View<'a> {
    active: ClauseArray<bool>,
    db: &'a ClauseStorage,
}

impl View<'_> {
    pub fn del(&mut self, clause: Clause) {
        self.active[clause] = false;
    }

    pub fn add(&mut self, clause: Clause) {
        self.active[clause] = true;
    }

    pub fn is_active(&self, clause: Clause) -> bool {
        self.active[clause]
    }

    pub fn clauses(&self) -> impl Iterator<Item = Clause> + '_ {
        (0..self.db.number_of_clauses()).filter_map(|i| {
            let clause = Clause { index: i };
            if self.active[clause] {
                Some(clause)
            } else {
                None
            }
        })
    }

    pub fn clause(&self, clause: Clause) -> &[Literal] {
        self.db.clause(clause)
    }
}

/// The clause database stores all clauses that exist within the proof and formula.
#[derive(Debug)]
pub struct ClauseStorage {
    literals: Vec<Literal>,
    ranges: Vec<Range<usize>>,
    max_literal: i32,
}

impl ClauseStorage {
    pub fn new_assignment(&self) -> Assignment {
        Assignment {
            trace: vec![],
            inner: LiteralSet {
                inner: self.literal_array(),
            },
        }
    }

    pub fn literal_array<T: Default + Clone>(&self) -> LiteralArray<T> {
        LiteralArray {
            inner: vec![T::default(); (self.max_literal * 2 + 1) as usize],
            max_literal: self.max_literal,
        }
    }

    pub fn clause_array<T: Default + Clone>(&self) -> ClauseArray<T> {
        ClauseArray {
            inner: vec![T::default(); self.number_of_clauses()],
        }
    }

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
    pub fn clause(&self, clause: Clause) -> &[Literal] {
        let range = &self.ranges[clause.index];
        &self.literals[range.start..range.end]
    }

    pub fn extract_true_unit(&self, clause: Clause) -> Option<Literal> {
        let range = &self.ranges[clause.index];
        if range.end - range.start == 1 {
            Some(self.literals[range.start])
        } else {
            None
        }
    }

    pub fn is_empty(&self, clause: Clause) -> bool {
        self.ranges[clause.index].is_empty()
    }

    /// Marks the first n clauses as active
    pub fn partial_view(&self, n: usize) -> View {
        let mut active = self.clause_array();
        for i in 0..n {
            active[Clause { index: i }] = true;
        }
        View { db: &self, active }
    }
}

pub struct Builder {
    clauses: FxHashMap<BTreeSet<Literal>, Clause>,
    clause_db: ClauseStorage,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            clauses: FxHashMap::default(),
            clause_db: ClauseStorage {
                literals: vec![],
                ranges: vec![],
                max_literal: 0,
            },
        }
    }

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

    pub fn finish(mut self) -> ClauseStorage {
        self.clause_db.max_literal = self
            .clause_db
            .literals
            .iter()
            .map(|lit| lit.raw().abs())
            .max()
            .expect("clause storage cannot be empty");
        self.clause_db
    }
}
