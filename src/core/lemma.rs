use super::Clause;

#[derive(Clone)]
pub enum Lemma {
    Addition(Clause),
    Deletion(Clause),
}
