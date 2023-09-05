mod assignment;
mod clause;
mod clause_storage;
mod lemma;
mod literal;

pub use assignment::*;
pub use clause::*;
pub use clause_storage::*;
pub use lemma::*;
pub use literal::*;

#[derive(Debug)]
pub enum MaybeConflict {
    Conflict,
    NoConflict
}
