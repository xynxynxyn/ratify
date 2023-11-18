mod assignment;
mod literal;
pub mod storage;

use std::collections::BTreeSet;

pub use assignment::*;
pub use literal::*;

use self::storage::Clause;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Conflict {}

#[derive(Debug, Hash)]
pub enum RawLemma {
    Add(BTreeSet<Literal>),
    Del(BTreeSet<Literal>),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Lemma {
    Add(Clause),
    Del(Clause),
}
