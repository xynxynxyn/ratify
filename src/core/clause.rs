use itertools::Itertools;
use std::{collections::BTreeSet, fmt::Display};

use super::{Assignment, Literal};
/// A clause consists of a set of literals in disjunction.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Clause(BTreeSet<Literal>);

impl Clause {
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

    // Returns true if only a single literal is true or unknown in the clause.
    pub fn is_unit(&self, assignment: &Assignment) -> bool {
        self.literals()
            .filter(|lit| !assignment.has_literal(!*lit))
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

    pub fn is_trivial(&self) -> bool {
        let len = self.0.len();
        len <= 1
    }

    /// Return the single literal in the clause if the clause is a unit.
    pub fn unit(&self) -> Option<Literal> {
        if self.0.len() == 1 {
            self.0.first().copied()
        } else {
            None
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
