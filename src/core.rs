use itertools::Itertools;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt::Display,
    hash::Hash,
    ops::Not,
};

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

impl Not for &Literal {
    type Output = Literal;
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
    pub fn literals(&self) -> impl Iterator<Item = &Literal> {
        self.0.iter()
    }

    /// Check if the clause is the empty clause.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Check if the clause contains the specified literal. This does not return
    /// true if it contains the negated literal.
    pub fn has_literal(&self, literal: Literal) -> bool {
        self.0.contains(&literal)
    }

    /// Given an assignment of literals, give an evaluation of the clause. If a
    /// unit is encountered, return the unknown literal.
    pub fn eval(&self, assignment: &Assignment) -> Evaluation {
        if self.0.is_empty() {
            return Evaluation::False;
        }

        let mut assigned = 0;
        let mut last_unknown = self.0.first().expect("clause cannot be empty");
        for lit in &self.0 {
            if assignment.0.contains(lit) {
                return Evaluation::True;
            }
            if assignment.0.contains(&!lit) {
                assigned += 1;
            } else {
                last_unknown = lit;
            }
        }

        if assigned == self.0.len() {
            Evaluation::False
        } else if assigned == self.0.len() - 1 {
            Evaluation::Unit(*last_unknown)
        } else {
            Evaluation::Unknown
        }
    }

    // Returns true if only a single literal is true or unknown in the clause.
    pub fn is_unit(&self, assignment: &Assignment) -> bool {
        self.literals()
            .filter(|lit| !assignment.0.contains(&!**lit))
            .count()
            == 1
    }

    /// Create a new clause which is the resolvent of self and other on the
    /// provided literal. This does not check if self contains the literal and
    /// other contains the negated literal.
    pub fn resolve(&self, other: &Clause, literal: Literal) -> Self {
        let mut left = self.0.clone();
        let mut right = other.0.clone();
        left.remove(&literal);
        right.remove(&!literal);
        left.extend(right.into_iter());
        Clause(left)
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
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Assignment(HashSet<Literal>);

impl Assignment {
    /// Create a new empty assignment.
    pub fn new() -> Self {
        Assignment(HashSet::new())
    }

    /// Add a literal to the assignment.
    pub fn add_literal(&mut self, lit: Literal) {
        self.0.insert(lit);
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

#[derive(Clone)]
pub enum Lemma {
    Addition(Clause),
    Deletion(Clause),
}

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
