use std::{fmt::Display, ops::Not};

/// The symbol of a literal. The value inside may never be negated or be zero.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Symbol(i32);

impl From<Literal> for Symbol {
    fn from(value: Literal) -> Self {
        Symbol(value.0.abs())
    }
}

/// An instantiation of a symbol, can also be negated.
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
