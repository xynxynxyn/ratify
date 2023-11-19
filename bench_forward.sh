#!/bin/bash
echo "creating unsat proof"
glucose $1 -certified -certified-output=___proof.drat 1> /dev/null

echo "building release binary"
cargo build --release

ratify_cmd="./target/release/ratify $1 ___proof.drat $2"
rate_cmd="rate $1 ___proof.drat -f --skip-unit-deletions"
drat_trim_cmd="drat-trim $1 ___proof.drat -f"

hyperfine "$ratify_cmd" -n "ratify"\
        "$rate_cmd" -n "rate"\
        "$drat_trim_cmd" -n "drat-trim"\
        --warmup 1
