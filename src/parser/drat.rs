use anyhow::{anyhow, Result};
use nom::{
    bytes::complete::tag,
    character::complete::{multispace0, multispace1},
    combinator::opt,
    sequence::{pair, tuple},
    IResult, Parser,
};

use super::parse_clause;
use crate::common::RawLemma;

fn parse_lemma(input: &str) -> IResult<&str, RawLemma> {
    let (input, (del, clause)) = pair(
        opt(tuple((multispace0, tag("d"), multispace1))),
        parse_clause,
    )
    .parse(input)?;

    if del.is_some() {
        Ok((input, RawLemma::Del(clause)))
    } else {
        Ok((input, RawLemma::Add(clause)))
    }
}

pub fn parse(input: &str) -> Result<Vec<RawLemma>> {
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
