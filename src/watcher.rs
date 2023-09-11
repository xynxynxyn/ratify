use itertools::Itertools;
use std::cell::RefCell;

use crate::core::{Assignment, Clause, ClauseRef, ClauseStorage, Literal, Symbol};

#[derive(Debug)]
struct Entry {
    symbols: (Symbol, Symbol),
    literals: (Literal, Literal),
    c_ref: ClauseRef,
}

#[derive(Debug)]
pub struct Watcher {
    inner: RefCell<Vec<Option<Entry>>>,
    // TODO add another datastruct here to speed up watched_by?
}

impl Watcher {
    /// Create a new watcher from a given set of clauses
    pub fn new(clause_db: &ClauseStorage) -> Self {
        let mut inner = std::iter::repeat_with(|| None)
            .take(clause_db.size())
            .collect_vec();
        for c_ref in clause_db.all_clause_refs() {
            let clause = clause_db.get_any_clause(c_ref);
            if clause.is_trivial() {
                // ignore if the clause is trivial (unit or empty clause) as we
                // cannot extract at least two literals from it.
                continue;
            }

            // each clause after this must have at least 2 literals since
            // otherwise it would be trivial
            let lits = clause.literals().take(2).collect_vec();
            // make sure that there are exactly two literals to watch
            debug_assert!(lits.len() == 2);

            inner[c_ref.to_index()] = Some(Entry {
                symbols: (Symbol::from(*lits[0]), Symbol::from(*lits[1])),
                literals: (*lits[0], *lits[1]),
                c_ref,
            });
        }
        Watcher {
            inner: RefCell::new(inner),
        }
    }

    /// Get the two literals being watched by a clause. This returns None if the
    /// clause does not exist or is trivial.
    pub fn watches(&self, c_ref: ClauseRef) -> Option<(Literal, Literal)> {
        self.inner
            .borrow()
            .get(c_ref.to_index())
            .map(|opt| opt.as_ref().map(|e| e.literals))
            .flatten()
    }

    /// Return all the clause references that are currently watching a given
    /// symbol.
    pub fn watched_by(&self, sym: Symbol) -> impl Iterator<Item = ClauseRef> + '_ {
        // make this an iterator, check:
        // https://users.rust-lang.org/t/return-an-iterator-from-struct-in-refcell/86580/2
        struct CRefIter<'a> {
            rc: &'a Watcher,
            pos: usize,
            sym: Symbol,
        }

        impl<'a> Iterator for CRefIter<'a> {
            type Item = ClauseRef;
            fn next(&mut self) -> Option<Self::Item> {
                let inner = self.rc.inner.borrow();
                loop {
                    if self.pos >= inner.len() {
                        return None;
                    }
                    let pos = self.pos;
                    self.pos += 1;
                    if let Some(e) = &inner[pos] {
                        if e.symbols.0 == self.sym || e.symbols.1 == self.sym {
                            return Some(e.c_ref);
                        }
                    }
                }
            }
        }

        CRefIter {
            rc: self,
            pos: 0,
            sym,
        }
    }

    /// Mutates the watcher to unwatch the symbol for the given clause. A new
    /// unassigned literal in the clause will be watched instead. If there is no
    /// other unassigned literal, the other watched literal is instead returned.
    /// This indicates a unit under the given assignment and no mutation is
    /// done.
    pub fn update(
        &self,
        c_ref: ClauseRef,
        sym: Symbol,
        clause_db: &ClauseStorage,
        assg: &Assignment,
    ) -> Option<Literal> {
        if let Some(Some(e)) = self.inner.borrow_mut().get_mut(c_ref.to_index()) {
            // found the correct entry
            // get the other symbol that is being watched.
            let (other_lit, other_is_first) = if e.symbols.0 == sym {
                (e.literals.1, false)
            } else if e.symbols.1 == sym {
                (e.literals.0, true)
            } else {
                // if the clause does not watch the given symbol, early
                // return
                return None;
            };

            let clause = clause_db.get_any_clause(c_ref);
            if let Some(next_unassigned) = find_next_unassigned(clause, assg, other_lit) {
                if other_is_first {
                    // the first literal in the tuple is the other literal
                    e.symbols = (e.symbols.0, Symbol::from(next_unassigned));
                    e.literals = (e.literals.0, next_unassigned);
                } else {
                    // the second literal in the tuple is the other literal
                    e.symbols = (Symbol::from(next_unassigned), e.symbols.1);
                    e.literals = (next_unassigned, e.literals.1);
                }
                return None;
            } else {
                // didn't find a new unassigned literal, this must be a unit
                // do not mutate anything, instead return the other literal
                return Some(other_lit);
            }
        }
        None
    }
}

fn find_next_unassigned(clause: &Clause, assg: &Assignment, except: Literal) -> Option<Literal> {
    clause
        .literals()
        .filter(|&lit| *lit != except && !assg.has_symbol(Symbol::from(*lit)))
        .next()
        .copied()
}
