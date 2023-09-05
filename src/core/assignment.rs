use std::fmt::Display;

use itertools::Itertools;

use super::{Clause, Literal, MaybeConflict};
/// A collection of literals which should all be true. Mainly used to evaluate a
/// clause.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Assignment(Vec<Literal>);

impl Assignment {
    /// Create a new empty assignment.
    pub fn new() -> Self {
        Assignment(Vec::new())
    }

    /// Add a literal to the assignment.
    pub fn assign(&mut self, lit: Literal) -> MaybeConflict {
        if self.0.contains(&!lit) {
            MaybeConflict::Conflict
        } else {
            if !self.0.contains(&lit) {
                self.0.push(lit);
            }
            MaybeConflict::NoConflict
        }
    }

    pub fn unassign(&mut self, lit: &Literal) {
        for (i, l) in self.0.iter().enumerate() {
            if lit == l {
                self.0.swap_remove(i);
                return;
            }
        }
    }

    pub fn has_literal(&self, lit: &Literal) -> bool {
        self.0.contains(lit)
    }

    pub fn get_literal(&self, index: usize) -> Literal {
        self.0[index]
    }

    pub fn len(&self) -> usize {
        self.0.len()
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
