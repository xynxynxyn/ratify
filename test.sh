#!/bin/bash

glucose $1 -certified -certified-output=___proof.drat 1> /dev/null
echo "drat-trim"
drat-trim $1 ___proof.drat -u -f
echo -e "\nratify"
cargo run --release -- $1 ___proof.drat -p -r
