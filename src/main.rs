/// Implementation of the algorithms.
mod algo;
/// The core module contains the core data structures, such as Formula, Clause
/// and Literals. It also includes the interfaces with which the data is
/// accessed.
mod core;
/// The parser module contains anything related to parsing. Mainly DIMACS and
/// DRAT in both text and binary form.
mod parser;

use crate::{
    algo::{forward_validate, Verdict},
    core::ClauseStorage,
};
use anyhow::{bail, Result};
use std::env;

fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        bail!("usage: ratify [DIMACS] [DRAT]");
    }

    // parse the input files
    let clauses = parser::cnf::parse(&std::fs::read_to_string(&args[1])?)?;
    let lemmas = parser::drat::parse(&std::fs::read_to_string(&args[2])?)?;

    // create the clause storage
    let mut clause_db = ClauseStorage::from_iter(clauses.into_iter());

    // validate the proof against the clauses
    let res = forward_validate(&mut clause_db, &lemmas);
    println!("{}", res);
    match res {
        Verdict::RefutationVerified => Ok(()),
        Verdict::RefutationRefuted => bail!("refutation not verified"),
        Verdict::NoConflict => bail!("no conflict detected"),
    }
}
