use std::{fmt::Display, ops::Not};

/// The smallest data type representing a single variable.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Literal(i32);

impl Literal {
    pub fn abs(&self) -> Literal {
        Literal(self.0.abs())
    }

    /// Check if self and other are the same symbol. Negation does not apply
    /// here.
    pub fn equal(&self, other: &Literal) -> bool {
        self.abs() == other.abs()
    }
}

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
