use std::fmt::Display;

use itertools::Itertools;

use super::{Conflict, Literal, storage::LiteralSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub(super) trace: Vec<Literal>,
    pub(super) inner: LiteralSet,
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
        if self.inner.contains(-literal) {
            Check::Conflicts
        } else if self.inner.contains(literal) {
            Check::Assigned
        } else {
            Check::Unassigned
        }
    }

    pub fn nth_lit(&self, n: usize) -> Literal {
        self.trace[n]
    }

    // assign a new literal
    // this may be slow since checking if it already exists is O(n) instead of O(1)
    pub fn try_assign(&mut self, literal: Literal) -> Result<(), Conflict> {
        match self.check(literal) {
            Check::Assigned => Ok(()),
            Check::Conflicts => Err(Conflict {
                _caused_by: literal,
            }),
            Check::Unassigned => {
                self.trace.push(literal);
                self.inner.insert(literal);
                Ok(())
            }
        }
    }

    pub fn force_assign(&mut self, literal: Literal) {
        match self.check(literal) {
            Check::Assigned => (),
            _ => {
                self.trace.push(literal);
                self.inner.insert(literal);
            }
        }
    }

    pub fn is_true(&self, literal: Literal) -> bool {
        matches!(self.check(literal), Check::Assigned)
    }

    pub fn rollback_to(&mut self, rollback: Rollback) {
        let cut = self.trace.split_off(rollback.len);
        for lit in cut {
            self.inner.remove(lit);
        }
    }

    pub fn rollback_point(&self) -> Rollback {
        Rollback {
            len: self.trace.len(),
        }
    }

    pub fn len(&self) -> usize {
        self.trace.len()
    }
}

impl Display for Assignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            self.trace.iter().map(|lit| lit.to_string()).join(",")
        )
    }
}

#[cfg(test)]
mod test {
    use crate::common::{assignment::Check, storage::Builder, Literal};

    use super::Assignment;

    fn init() -> Assignment {
        let mut b = Builder::new();
        b.add_clause(vec![-8].into_iter().map(|i| Literal::from(i)).collect());
        let db = b.finish();
        db.new_assignment()
    }

    #[test]
    fn test() {
        let mut ass = init();

        for c in vec![1, 2, 3, -4, -5, 7, 8].into_iter().map(Literal::from) {
            ass.force_assign(c)
        }

        assert!(ass.try_assign(Literal::from(-1)).is_err());
        let reference = ass.clone();
        let r = ass.rollback_point();

        assert!(matches!(ass.check(Literal::from(-6)), Check::Unassigned));
        assert!(matches!(ass.check(Literal::from(6)), Check::Unassigned));

        assert!(matches!(ass.check(Literal::from(2)), Check::Assigned));
        assert!(matches!(ass.check(Literal::from(-5)), Check::Assigned));

        assert!(matches!(ass.check(Literal::from(-1)), Check::Conflicts));
        assert!(matches!(ass.check(Literal::from(5)), Check::Conflicts));

        assert!(ass.try_assign(Literal::from(6)).is_ok());
        assert!(ass.try_assign(Literal::from(-6)).is_err());

        ass.rollback_to(r);
        assert_eq!(ass, reference);
    }
}
