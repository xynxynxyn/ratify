/// The core module contains the core data structures, such as Formula, Clause
/// and Literals. It also includes the interfaces with which the data is
/// accessed.
mod core;
/// The parser module contains anything related to parsing. Mainly DIMACS and
/// DRAT in both text and binary form.
mod parser;
/// Implementation of the validation algorithms.
mod validator;
/// Trait and implementation of the watcher struct for fast unit propagation.
mod watcher;

use crate::{
    core::{ClauseStorage, Lemma, RefLemma},
    validator::{BackwardValidator, ForwardValidator, Validator, Verdict},
};
use anyhow::{bail, Result};
use clap::Parser;
use itertools::Itertools;

#[derive(Parser, Debug)]
pub struct Features {
    #[arg(short, long)]
    /// Apply all deletions as they occur. This often invalidates the proof as
    /// unit deletions are common and do not preserve satisfiability.
    strict: bool,
    #[arg(short, long)]
    /// Only do RUP checking, skip any RAT check and assume invalid for those
    /// lemmas.
    rup_only: bool,
    #[arg(short, long)]
    progress: bool,
    #[arg(short, long)]
    forward: bool,
    cnf: String,
    proof: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let features = Features::parse();

    // parse the input files
    let (_, clauses) = parser::cnf::parse(&std::fs::read_to_string(&features.cnf)?)?;
    let lemmas = parser::drat::parse(&std::fs::read_to_string(&features.proof)?)?;

    let mut clause_db = ClauseStorage::with_capacity(clauses.len() + lemmas.len());
    // populate clause storage with formula clauses
    clause_db.add_from_iter(clauses.into_iter(), true);
    // add proof lemma clauses to storage and convert them to RefLemma's that use ClauseRef instead
    // of clauses.
    let lemmas = lemmas
        .into_iter()
        .map(|lemma| match lemma {
            Lemma::Addition(c) => RefLemma::Addition(clause_db.add_clause(c, false)),
            Lemma::Deletion(c) => RefLemma::Deletion(clause_db.add_clause(c, false)),
        })
        .collect_vec();

    // validate the proof against the clauses
    let res = if features.forward {
        ForwardValidator::init(clause_db, features)?.validate(lemmas)
    } else {
        BackwardValidator::init(clause_db, features)?.validate(lemmas)
    };

    println!("{}", res);
    match res {
        Verdict::RefutationVerified => Ok(()),
        Verdict::EarlyRefutation => bail!("early refutation detected"),
        Verdict::RefutationRefuted => bail!("refutation not verified"),
        Verdict::NoConflict => bail!("no conflict detected"),
    }
}
