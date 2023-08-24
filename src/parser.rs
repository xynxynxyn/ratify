pub mod dimacs;
pub mod drat;
use crate::core::{Clause, Literal};

use anyhow::bail;
use nom::{
    bytes::complete::tag,
    character::complete::{digit1, multispace1},
    combinator::{map_res, opt, recognize},
    multi::separated_list1,
    sequence::pair,
    IResult, Parser,
};

fn parse_i32(input: &str) -> IResult<&str, i32> {
    map_res(recognize(pair(opt(tag("-")), digit1)), str::parse).parse(input)
}

fn parse_clause(input: &str) -> IResult<&str, Clause> {
    map_res(separated_list1(multispace1, parse_i32), |ids| {
        match ids.split_last() {
            Some((0, rest)) => Ok(Clause::from_iter(rest.iter().map(|&i| Literal::from(i)))),
            _ => bail!("invalid clause '{}'", input),
        }
    })
    .parse(input)
}
