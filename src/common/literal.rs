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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralMap<T> {
    pub(super) inner: Vec<T>,
    pub(super) max_literal: i32,
}

impl<T> Index<Literal> for LiteralMap<T> {
    type Output = T;
    #[inline]
    fn index(&self, index: Literal) -> &Self::Output {
        let index = index.raw();
        if index < 0 {
            &self.inner[(index.abs() + self.max_literal) as usize]
        } else {
            &self.inner[index as usize]
        }
    }
}

impl<T> IndexMut<Literal> for LiteralMap<T> {
    #[inline]
    fn index_mut(&mut self, index: Literal) -> &mut Self::Output {
        let index = index.raw();
        if index < 0 {
            &mut self.inner[(index.abs() + self.max_literal) as usize]
        } else {
            &mut self.inner[index as usize]
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LiteralSet {
    pub(super) inner: LiteralMap<bool>,
}

impl LiteralSet {
    pub fn insert(&mut self, lit: Literal) -> bool {
        let already_present = self.contains(lit);
        self.inner[lit] = true;
        !already_present
    }

    pub fn contains(&self, lit: Literal) -> bool {
        self.inner[lit]
    }

    pub fn remove(&mut self, lit: Literal) -> bool {
        let already_present = self.contains(lit);
        self.inner[lit] = false;
        already_present
    }
}
