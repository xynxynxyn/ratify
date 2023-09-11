use std::{collections::HashSet, fmt::Display};

use itertools::Itertools;

use super::{Clause, Literal, Symbol};
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
    pub fn assign(&mut self, lit: Literal) -> bool {
        // make sure that this does not result in conflict
        debug_assert!(!self.has_literal(!lit));
        self.0.insert(lit)
    }

    pub fn conflicts(&self, lit: Literal) -> bool {
        self.0.contains(&!lit)
    }

    pub fn has_literal(&self, lit: Literal) -> bool {
        self.0.contains(&lit)
    }

    /// Check if the given symbol exists in the assignment.
    pub fn has_symbol(&self, sym: Symbol) -> bool {
        self.0.iter().any(|lit| Symbol::from(*lit) == sym)
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
