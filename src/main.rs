mod common;
mod forward;

mod parser;

use anyhow::Result;
use clap::Parser;
use common::storage::{ClauseStorage, View};
use fxhash::FxHashSet;
use itertools::Itertools;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::common::{storage, Lemma, RawLemma};

#[derive(Parser, Debug)]
pub struct Flags {
    #[arg(short, long)]
    /// Only check lemmas for the RUP property instead of RAT if the RUP check fails.
    rup_only: bool,
    #[arg(short, long)]
    /// Show the progress bar during verification to indicate how many proof steps have been
    /// processed.
    progress: bool,
    #[arg(long)]
    /// Skip all deletion steps in a proof.
    ignore_deletions: bool,
    #[arg(short, long)]
    /// Specify whether a mutable or immutable clause storage should be used.
    /// A mutable storage has higher performance in single threaded environments while an immutable
    /// storage can potentially be faster with multi threaded algorithms.
    mutating: bool,
    cnf: String,
    proof: String,
}

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    let flags = Flags::parse();

    let (_, formula) = parser::cnf::parse(&std::fs::read_to_string(&flags.cnf)?)?;
    let lemmas = parser::drat::parse(&std::fs::read_to_string(&flags.proof)?)?;

    let mut db_builder = storage::Builder::new();
    let mut seen = FxHashSet::default();
    let formula_clauses = formula.len();
    for c in formula {
        let clause = db_builder.add_clause(c);
        seen.insert(clause);
    }

    // convert the lemmas to clause ids
    // we also remove add lemmas if they add the same clause but it has not been deleted in between
    // the two additions
    // if we ignore deletions we also remove those duplicate additions
    let proof = lemmas
        .into_iter()
        .filter_map(|l| match l {
            RawLemma::Add(c) => {
                let clause = db_builder.add_clause(c);
                if !seen.insert(clause) {
                    tracing::warn!("lemma {} is added more than once", clause);
                    // we want to ignore duplicate additions
                    return None;
                }
                Some(Lemma::Add(clause))
            }
            RawLemma::Del(c) => {
                if flags.ignore_deletions {
                    None
                } else {
                    let clause = db_builder.add_clause(c);
                    if !seen.remove(&clause) {
                        tracing::warn!("lemma {} was not added before it was deleted", clause);
                        // if a lemma is to be deleted even though it does not exist we ignore that
                        // deletion step
                        None
                    } else {
                        Some(Lemma::Del(clause))
                    }
                }
            }
        })
        .collect_vec();
    let clause_db = db_builder.finish();

    // mark the formula clauses as active
    let db_view = clause_db.partial_view(formula_clauses);

    if flags.mutating {
        forward::MutatingChecker::with_flags(flags).validate(clause_db, db_view, proof)?;
    } else {
        forward::Checker::with_flags(flags).validate(clause_db, db_view, proof)?;
    }

    println!("s VERIFIED");
    Ok(())
}

trait Validator {
    fn with_flags(flags: Flags) -> Self;
    fn validate(
        self,
        clause_db: ClauseStorage,
        db_view: View,
        proof: Vec<Lemma>,
    ) -> anyhow::Result<()>;
}
