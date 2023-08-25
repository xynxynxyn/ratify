use super::{parse_clause, parse_i32};
use crate::core::Clause;
use anyhow::{anyhow, Result};
use log::info;
use nom::{
    bytes::complete::tag,
    character::complete::{multispace0, multispace1},
    sequence::tuple,
    IResult, Parser,
};

struct Header {
    vars: i32,
    clauses: i32,
}

fn parse_header(input: &str) -> IResult<&str, Header> {
    info!("input: {}", input);
    let (input, _) =
        tuple((multispace0, tag("p"), multispace1, tag("cnf"), multispace1)).parse(input)?;
    info!("input: {}", input);
    let (input, (vars, _, clauses)) = tuple((parse_i32, multispace1, parse_i32)).parse(input)?;
    Ok((input, Header { vars, clauses }))
}

pub fn parse(input: &str) -> Result<Vec<Clause>> {
    info!("parsing cnf");
    let mut lines = input.lines().filter(|s| !s.starts_with('c'));
    if let Some(line) = lines.next() {
        let (_, header) = parse_header(line).map_err(|_| anyhow!("invalid dimacs header"))?;
        info!("{} variables and {} clauses", header.vars, header.clauses);
    }

    let clauses = lines
        .into_iter()
        .map(|line| {
            parse_clause(line)
                .map(|(_, clause)| clause)
                .map_err(|_| anyhow!("invalid clause '{}'", line))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(clauses)
}
