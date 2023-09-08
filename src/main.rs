/// The core module contains the core data structures, such as Formula, Clause
/// and Literals. It also includes the interfaces with which the data is
/// accessed.
mod core;
/// The parser module contains anything related to parsing. Mainly DIMACS and
/// DRAT in both text and binary form.
mod parser;
/// Implementation of the validation algorithms.
mod validator;

use crate::validator::{validate, Verdict};
use anyhow::{bail, Result};
use clap::Parser;

#[derive(Parser, Debug)]
struct Features {
    #[arg(short, long)]
    /// Apply all deletions as they occur. This often invalidates the proof as
    /// unit deletions are common and do not preserve satisfiability.
    strict: bool,
    #[arg(short, long)]
    /// Only do RUP checking, skip any RAT check and assume invalid for those
    /// lemmas.
    rup_only: bool,
    cnf: String,
    proof: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let features = Features::parse();

    // parse the input files
    let (_, clauses) = parser::cnf::parse(&std::fs::read_to_string(features.cnf)?)?;
    let lemmas = parser::drat::parse(&std::fs::read_to_string(features.proof)?)?;

    // validate the proof against the clauses
    let res = validate(clauses, lemmas);
    println!("{}", res);
    match res {
        Verdict::RefutationVerified => Ok(()),
        Verdict::EarlyRefutation => bail!("early refutation detected"),
        Verdict::RefutationRefuted => bail!("refutation not verified"),
        Verdict::NoConflict => bail!("no conflict detected"),
    }
}
