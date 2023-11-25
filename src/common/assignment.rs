use std::fmt::Display;

use itertools::Itertools;

use super::{
    storage::{Clause, ClauseStorage, LiteralSet},
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
    pub fn try_assign(&mut self, literal: Literal) -> Result<bool, Conflict> {
        // check if the negation is assigned
        if self.inner.contains(-literal) {
            Err(Conflict {})
        } else if self.inner.insert(literal) {
            // the literal has not been assigned already, add it to the trace
            self.trace.push(literal);
            Ok(true)
        } else {
            Ok(false)
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
        for &lit in &self.trace[rollback_point.len..] {
            self.inner.remove(lit);
        }

        self.trace.truncate(rollback_point.len)
    }

    pub fn is_satisfied(&self, clause: Clause, clause_db: &ClauseStorage) -> bool {
        clause_db
            .clause(clause)
            .into_iter()
            .any(|&lit| self.is_true(lit))
    }

    pub fn trace_len(&self) -> usize {
        self.trace.len()
    }

    pub fn nth_lit(&self, n: usize) -> Literal {
        self.trace[n]
    }
}

impl Display for Assignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            self.trace
                .iter()
                .sorted()
                .map(|lit| lit.to_string())
                .join(",")
        )
    }
}
