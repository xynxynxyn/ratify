use std::{
    fmt::Display,
    num::NonZeroI32,
    ops::{Index, IndexMut, Neg},
};

/// A literal represented by an integer
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Literal {
    // We choose a nonzeroi32 to optimize nullable data structures
    inner: NonZeroI32,
}

impl Literal {
    /// Compares to literals and returns true if they use the same variable
    pub fn matches(self, other: Self) -> bool {
        self.inner.abs() == other.inner.abs()
    }

    pub fn abs(mut self) -> Self {
        self.inner = self.inner.abs();
        self
    }
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

#[derive(Debug)]
pub struct LiteralMap<T> {
    inner: Vec<T>,
}

impl<T> Index<Literal> for LiteralMap<T> {
    type Output = T;
    fn index(&self, index: Literal) -> &Self::Output {
        let mut index = index.raw();
        index = if index < 0 { index.abs() * 2 } else { index };
        &self.inner[index as usize]
    }
}

impl<T> IndexMut<Literal> for LiteralMap<T> {
    fn index_mut(&mut self, index: Literal) -> &mut Self::Output {
        let mut index = index.raw();
        index = if index < 0 { index.abs() * 2 } else { index };
        &mut self.inner[index as usize]
    }
}
