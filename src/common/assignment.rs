use super::{
    storage::{ClauseStorage, LiteralSet},
    Conflict, Literal,
};

#[derive(Debug, Copy, Clone)]
pub struct Rollback {
    len: usize,
}

pub struct Assignment {
    inner: LiteralSet,
    trace: Vec<Literal>,
}

impl Assignment {
    /// Create a new assignment from a clause storage
    pub fn new(clause_db: &ClauseStorage) -> Self {
        Assignment {
            inner: LiteralSet {
                inner: clause_db.literal_array(),
            },
            trace: vec![],
        }
    }

    /// Find the next literal out of a list of literals which is either unassigned or true.
    pub fn find_next_true_or_unassigned(
        &self,
        literals: &[Literal],
        except: Literal,
    ) -> Option<Literal> {
        literals
            .iter()
            .find(|&&lit| !self.inner.contains(-lit) && lit != except)
            .copied()
    }

    /// Try adding the literal to the assignment. If it is already assigned nothing happens. If it
    /// is falsified an error with a conflict is returned.
    pub fn try_assign(&mut self, literal: Literal) -> Result<(), Conflict> {
        // check if the negation is assigned
        if self.inner.contains(-literal) {
            Err(Conflict {})
        } else if self.inner.contains(literal) {
            Ok(())
        } else {
            self.inner.insert(literal);
            self.trace.push(literal);
            Ok(())
        }
    }

    pub fn rollback_point(&self) -> Rollback {
        Rollback {
            len: self.trace.len(),
        }
    }

    pub fn is_true(&self, literal: Literal) -> bool {
        self.inner.contains(literal)
    }

    pub fn rollback(&mut self, rollback_point: Rollback) {
        let cut = self.trace.split_off(rollback_point.len);
        for lit in cut {
            self.inner.remove(lit);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.trace.is_empty()
    }

    pub fn trace_len(&self) -> usize {
        self.trace.len()
    }

    pub fn nth_lit(&self, n: usize) -> Literal {
        self.trace[n]
    }
}
