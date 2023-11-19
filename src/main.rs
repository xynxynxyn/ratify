mod common;
mod forward;

mod parser;

use anyhow::Result;
use clap::Parser;
use common::storage::{ClauseStorage, View};
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
    skip_deletions: bool,
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
    let formula_clauses = formula.len();
    for c in formula {
        let _ = db_builder.add_clause(c);
    }
    let proof = lemmas
        .into_iter()
        .map(|l| match l {
            RawLemma::Add(c) => Lemma::Add(db_builder.add_clause(c)),
            RawLemma::Del(c) => Lemma::Del(db_builder.get_clause(c)),
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
