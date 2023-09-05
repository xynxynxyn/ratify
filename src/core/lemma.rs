use super::{Clause, ClauseRef};

#[derive(Clone)]
pub enum Lemma {
    Addition(Clause),
    Deletion(Clause),
}

#[derive(Clone, Copy)]
pub enum RefLemma {
    Addition(ClauseRef),
    Deletion(ClauseRef)
}
