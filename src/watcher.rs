use itertools::Itertools;
use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
};

use crate::core::{Assignment, Clause, ClauseRef, ClauseStorage, Literal, Symbol};

#[derive(Debug)]
struct Entry {
    symbols: (Symbol, Symbol),
    literals: (Literal, Literal),
}

#[derive(Debug)]
pub struct Watcher {
    mappings: RefCell<Vec<Option<Entry>>>,
    watching: RefCell<HashMap<Symbol, BTreeSet<ClauseRef>>>,
    empty: BTreeSet<ClauseRef>,
}

impl Watcher {
    /// Create a new watcher from a given set of clauses
    pub fn new(clause_db: &ClauseStorage) -> Self {
        let mut mappings = std::iter::repeat_with(|| None)
            .take(clause_db.size())
            .collect_vec();
        let mut watching = HashMap::new();
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
            let sym_0 = Symbol::from(*lits[0]);
            let sym_1 = Symbol::from(*lits[1]);

            mappings[c_ref.to_index()] = Some(Entry {
                symbols: (sym_0, sym_1),
                literals: (*lits[0], *lits[1]),
            });
            watching
                .entry(sym_0)
                .or_insert(BTreeSet::new())
                .insert(c_ref);
            watching
                .entry(sym_1)
                .or_insert(BTreeSet::new())
                .insert(c_ref);
        }
        Watcher {
            mappings: RefCell::new(mappings),
            watching: RefCell::new(watching),
            empty: BTreeSet::new(),
        }
    }

    /// Get the two literals being watched by a clause. This returns None if the
    /// clause does not exist or is trivial.
    pub fn watches(&self, c_ref: ClauseRef) -> Option<(Literal, Literal)> {
        self.mappings
            .borrow()
            .get(c_ref.to_index())
            .map(|opt| opt.as_ref().map(|e| e.literals))
            .flatten()
    }

    /// Return all the clause references that are currently watching a given
    /// symbol.
    pub fn watched_by(&self, sym: Symbol) -> impl Iterator<Item = ClauseRef> + '_ {
        self.watching
            .borrow()
            .get(&sym)
            .unwrap_or(&self.empty)
            .iter()
            .cloned()
            .collect_vec()
            .into_iter()
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
        let mut mappings = self.mappings.borrow_mut();
        let mut watching = self.watching.borrow_mut();
        let entry = mappings.get_mut(c_ref.to_index());
        if let Some(Some(e)) = entry {
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
                let new_symbol = Symbol::from(next_unassigned);
                let old_symbol = if other_is_first {
                    // the first literal in the tuple is the other literal
                    e.symbols = (e.symbols.0, new_symbol);
                    e.literals = (e.literals.0, next_unassigned);
                    e.symbols.1
                } else {
                    // the second literal in the tuple is the other literal
                    e.symbols = (new_symbol, e.symbols.1);
                    e.literals = (next_unassigned, e.literals.1);
                    e.symbols.0
                };

                // we unassign the old symbol from the watch tracker and reassign it to the new symbol
                watching
                    .entry(old_symbol)
                    .or_insert(BTreeSet::new())
                    .remove(&c_ref);
                watching
                    .entry(new_symbol)
                    .or_insert(BTreeSet::new())
                    .insert(c_ref);
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

// find the next literal that is not falsified
fn find_next_unassigned(clause: &Clause, assg: &Assignment, except: Literal) -> Option<Literal> {
    clause
        .literals()
        .filter(|&lit| *lit != except && !assg.has_literal(!*lit))
        .next()
        .copied()
}
