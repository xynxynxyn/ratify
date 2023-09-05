use anyhow::{anyhow, Result};
use log::info;
use nom::{
    bytes::complete::tag,
    character::complete::{multispace0, multispace1},
    combinator::opt,
    sequence::{pair, tuple},
    IResult, Parser,
};

use super::parse_clause;
use crate::core::Lemma;

fn parse_lemma(input: &str) -> IResult<&str, Lemma> {
    let (input, (del, clause)) = pair(
        opt(tuple((multispace0, tag("d"), multispace1))),
        parse_clause,
    )
    .parse(input)?;

    if del.is_some() {
        Ok((input, Lemma::Deletion(clause)))
    } else {
        Ok((input, Lemma::Addition(clause)))
    }
}

pub fn parse(input: &str) -> Result<Vec<Lemma>> {
    info!("parsing drat proof");
    input
        .lines()
        .filter(|s| !s.starts_with('c'))
        .map(|line| {
            parse_lemma(line)
                .map(|(_, lemma)| lemma)
                .map_err(|_| anyhow!("invalid lemma"))
        })
        .collect::<Result<Vec<_>>>()
}
