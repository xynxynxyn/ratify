use std::{fmt::Display, num::NonZeroI32, ops::Neg};

/// A literal represented by an integer
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Literal {
    // We choose a nonzeroi32 to optimize nullable data structures
    inner: NonZeroI32,
}

impl Literal {
    pub fn raw(&self) -> i32 {
        i32::from(self.inner)
    }
}

impl Neg for Literal {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        self.inner = -self.inner;
        self
    }
}

impl Default for Literal {
    fn default() -> Self {
        Literal {
            inner: NonZeroI32::new(1).unwrap(),
        }
    }
}

impl From<i32> for Literal {
    fn from(value: i32) -> Self {
        Literal {
            inner: NonZeroI32::new(value).expect("cannot create literal with id 0"),
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}
