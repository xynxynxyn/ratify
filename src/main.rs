mod common;
mod forward;

mod parser;

use std::collections::BTreeSet;

use anyhow::Result;
use clap::Parser;
use common::storage::{Builder, ClauseStorage, View};
use fxhash::FxHashMap;
use itertools::Itertools;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::common::{
    storage::{self, Clause},
    Lemma, Literal, RawLemma,
};

#[derive(clap::ValueEnum, Clone, Debug)]
enum Mode {
    Mutating,
    Immutable,
    Naive,
}

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
    #[arg(short, long, value_enum, default_value_t = Mode::Mutating)]
    /// The type of propagator that should be used. Options are Mutating, Immutable and Naive.
    /// Mutating will modify the underlying clause storage for efficiency while the immutable
    /// version keeps it in tact and has a more complex structure. Naive does not make use of
    /// watchlists and is thus very slow.
    mode: Mode,
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

    let proof = preprocess(formula, lemmas, &mut db_builder);
    let clause_db = db_builder.finish();

    // mark the formula clauses as active
    let db_view = clause_db.partial_view(formula_clauses);

    match flags.mode {
        Mode::Mutating => {
            forward::MutatingChecker::init(flags, clause_db, db_view).validate(proof)?
        }
        Mode::Immutable => {
            forward::ConstChecker::init(flags, clause_db, db_view).validate(proof)?
        }
        Mode::Naive => forward::NaiveChecker::init(flags, clause_db, db_view).validate(proof)?,
    }

    println!("s VERIFIED");
    Ok(())
}

trait Validator {
    fn init(flags: Flags, clause_db: ClauseStorage, db_view: View) -> Self;
    fn validate(self, proof: Vec<Lemma>) -> anyhow::Result<()>;
}

// Adds all the clauses from the original formula and the proof to the builder. The lemmas of the
// proof are converted to lemmas containing clause references and returned.
fn preprocess(
    formula: Vec<BTreeSet<Literal>>,
    proof: Vec<RawLemma>,
    builder: &mut Builder,
) -> Vec<Lemma> {
    let mut seen: FxHashMap<Clause, i32> = FxHashMap::default();

    for c in formula {
        let clause = builder.add_clause(c);
        *seen.entry(clause).or_default() += 1;
    }

    proof
        .into_iter()
        .enumerate()
        .filter_map(|(i, raw_lemma)| match raw_lemma {
            RawLemma::Add(c) => {
                let clause = builder.add_clause(c);
                let entry = seen.entry(clause).or_default();
                if *entry > 0 {
                    tracing::warn!("ignoring proof step {} addition of duplicate clause", i);
                    // The clause has already been added, increment the appearances, but do not add
                    // a duplicate
                    *entry += 1;
                    None
                } else {
                    // The clause has not been added yet, keep the proof step
                    *entry += 1;
                    Some(Lemma::Add(clause))
                }
            }
            RawLemma::Del(c) => {
                let clause = builder.add_clause(c);
                let entry = seen.entry(clause).or_default();
                // TODO maybe theres something we can do here to check if the clause has never
                // been added before and then we revert adding this clause to the database
                if *entry < 1 {
                    // The clause has not been added before it is deleted, ignore this step
                    tracing::warn!("ignoring proof step {} deletion of non existing clause", i);
                    None
                } else {
                    *entry -= 1;
                    if *entry == 0 {
                        // All instances of the clause were removed, actually keep the delete
                        // instruction then
                        Some(Lemma::Del(clause))
                    } else {
                        tracing::warn!("ignoring proof step {} deletion of duplicate clause", i);
                        None
                    }
                }
            }
        })
        .collect_vec()
}
