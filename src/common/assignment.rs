use super::{Conflict, Literal};

#[derive(Debug, Default)]
pub struct Assignment {
    literals: Vec<Literal>,
}

#[derive(Debug)]
enum Check {
    Assigned,
    Conflicts,
    Unassigned,
}

#[derive(Debug, Copy, Clone)]
pub struct Rollback {
    len: usize,
}

impl Assignment {
    fn check(&self, literal: Literal) -> Check {
        let neg_literal = -literal;
        for &lit in &self.literals {
            if lit == literal {
                return Check::Assigned;
            } else if lit == neg_literal {
                return Check::Conflicts;
            }
        }

        Check::Unassigned
    }

    pub fn nth_lit(&self, n: usize) -> Literal {
        self.literals[n]
    }

    // assign a new literal
    // this may be slow since checking if it already exists is O(n) instead of O(1)
    pub fn try_assign(&mut self, literal: Literal) -> Result<(), Conflict> {
        match self.check(literal) {
            Check::Assigned => Ok(()),
            Check::Conflicts => Err(Conflict { caused_by: literal }),
            Check::Unassigned => {
                self.literals.push(literal);
                Ok(())
            }
        }
    }

    pub fn force_assign(&mut self, literal: Literal) {
        match self.check(literal) {
            Check::Assigned => (),
            _ => self.literals.push(literal),
        }
    }

    pub fn is_true(&self, literal: Literal) -> bool {
        matches!(self.check(literal), Check::Assigned)
    }

    pub fn rollback_to(&mut self, rollback: Rollback) {
        self.literals.truncate(rollback.len)
    }

    pub fn rollback_point(&self) -> Rollback {
        Rollback {
            len: self.literals.len(),
        }
    }

    pub fn len(&self) -> usize {
        self.literals.len()
    }
}
