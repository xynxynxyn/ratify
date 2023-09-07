use std::{collections::HashSet, fmt::Display};

use itertools::Itertools;

use super::{Clause, Literal, MaybeConflict};
/// A collection of literals which should all be true. Mainly used to evaluate a
/// clause.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Assignment(HashSet<Literal>);

impl Assignment {
    /// Create a new empty assignment.
    pub fn new() -> Self {
        Assignment(HashSet::new())
    }

    /// Add a literal to the assignment. This will indicate a conflict if it
    /// exists.
    pub fn assign(&mut self, lit: Literal) -> MaybeConflict {
        if self.0.insert(lit) {
            MaybeConflict::NoConflict
        } else {
            MaybeConflict::Conflict
        }
    }

    pub fn has_literal(&self, lit: &Literal) -> bool {
        self.0.contains(lit)
    }

    pub fn literals(&self) -> impl Iterator<Item = &Literal> {
        self.0.iter()
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
