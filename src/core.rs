use itertools::Itertools;
use std::{collections::BTreeSet, fmt::Display, ops::Not};

/// The smallest data type representing a single variable.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Literal(i32);

impl From<i32> for Literal {
    fn from(id: i32) -> Self {
        if id == 0 {
            panic!("literals cannot have 0 as their id");
        } else {
            Literal(id)
        }
    }
}

impl Not for Literal {
    type Output = Self;
    fn not(self) -> Self::Output {
        Literal(-self.0)
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A clause consists of a set of literals in disjunction.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct Clause(BTreeSet<Literal>);

pub enum Evaluation {
    Unit(Literal),
    True,
    False,
    Unknown,
}

impl Clause {
    /// Create the empty clause.
    pub fn empty() -> Self {
        Clause(BTreeSet::new())
    }

    /// Create a new clause from an iterator of literals.
    pub fn from_iter(literals: impl Iterator<Item = Literal>) -> Self {
        Clause(BTreeSet::from_iter(literals))
    }

    /// Return an iterator over all the literals present in the clause.
    fn literals(&self) -> impl Iterator<Item = &Literal> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Given an assignment of literals, give an evaluation of the clause.
    pub fn eval(&self, assignment: &Assignment) -> Evaluation {
        if assignment.0.is_disjoint(&self.0) {
            let negation = assignment.inverse();
            if negation.0.is_superset(&self.0) {
                Evaluation::False
            } else {
                let unassigned = self.0.difference(&negation.0).collect_vec();
                if unassigned.len() == 1 {
                    Evaluation::Unit(*unassigned[0])
                } else {
                    Evaluation::Unknown
                }
            }
        } else {
            Evaluation::True
        }
    }
}

impl Display for Clause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            Itertools::intersperse(self.0.iter().map(Literal::to_string), ", ".to_string())
                .collect::<String>()
        )
    }
}

/// A collection of literals which should all be true. Mainly used to evaluate a
/// clause.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct Assignment(BTreeSet<Literal>);

impl Assignment {
    pub fn add_literal(&mut self, lit: Literal) {
        self.0.insert(lit);
    }

    /// Negate every literal in the assignment.
    fn inverse(&self) -> Self {
        Assignment(self.0.iter().map(|&lit| !lit).collect())
    }
}

impl From<&Clause> for Assignment {
    /// Take all the literals from a clause and invert them.
    /// For example: (1 || 2 || !3) becomes (!1 && !2 && 3).
    fn from(clause: &Clause) -> Self {
        Assignment(clause.literals().map(|&lit| !lit).collect())
    }
}

impl Display for Assignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            Itertools::intersperse(self.0.iter().map(Literal::to_string), ", ".to_string())
                .collect::<String>()
        )
    }
}

pub enum Lemma {
    Addition(Clause),
    Deletion(Clause),
}

/// The purpose of this data structure is to efficiently store clauses, which
/// are a collection of literals. A variety of methods to easily and quickly
/// find relevant clauses should be provided.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct ClauseStorage(BTreeSet<Clause>);

impl ClauseStorage {
    /// Add the given clause to the storage.
    pub fn add_clause(&mut self, clause: Clause) {
        self.0.insert(clause);
    }

    pub fn from_iter(clauses: impl Iterator<Item = Clause>) -> Self {
        ClauseStorage(BTreeSet::from_iter(clauses))
    }

    /// Removes the clause which is equal to the one provided.
    pub fn del_clause(&mut self, clause: &Clause) {
        self.0.remove(clause);
    }

    pub fn clauses(&self) -> impl Iterator<Item = &Clause> {
        self.0.iter()
    }
}
