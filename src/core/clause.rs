use itertools::Itertools;
use std::{collections::BTreeSet, fmt::Display};

use super::{Assignment, Literal};
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
            if assignment.has_literal(lit) {
                return Evaluation::True;
            }
            if assignment.has_literal(&!lit) {
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
            .filter(|lit| !assignment.has_literal(&!*lit))
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
