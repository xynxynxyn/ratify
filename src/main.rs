/// rewriting everything
mod common;
mod forward_validate;
mod propagator;

mod parser;

use anyhow::Result;
use clap::Parser;
use itertools::Itertools;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::{
    common::{storage, Lemma, RawLemma},
    forward_validate::validate,
};

#[derive(Parser, Debug)]
pub struct Features {
    #[arg(short, long)]
    /// Only do RUP checking, skip any RAT check and assume invalid for those
    /// lemmas.
    rup_only: bool,
    #[arg(short, long)]
    progress: bool,
    #[arg(short, long)]
    forward: bool,
    #[arg(long)]
    skip_deletions: bool,
    cnf: String,
    proof: String,
}

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    let flags = Features::parse();

    let (_, formula) = parser::cnf::parse(&std::fs::read_to_string(&flags.cnf)?)?;
    let lemmas = parser::drat::parse(&std::fs::read_to_string(&flags.proof)?)?;

    let mut db_builder = storage::Builder::default();
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

    validate(&clause_db, db_view, proof)?;
    println!("valid");

    Ok(())
}
