use super::{Clause, ClauseRef};

#[derive(Clone)]
pub enum Lemma {
    Addition(Clause),
    Deletion(Clause),
}

#[derive(Clone, Copy)]
pub enum RefLemma {
    Addition(ClauseRef),
    Deletion(ClauseRef),
}

impl RefLemma {
    pub fn into_c_ref(self) -> ClauseRef {
        match self {
            RefLemma::Addition(c_ref) => c_ref,
            RefLemma::Deletion(c_ref) => c_ref,
        }
    }
}
