#!/bin/bash

glucose $1 -certified -certified-output=___proof.drat 1> /dev/null
echo "drat-trim"
time drat-trim $1 ___proof.drat -u -f
echo -e "\nratify"
cargo build --release
if [ -z $2 ]
then
        time ./target/release/ratify $1 ___proof.drat -p -r
else
        time ./target/release/ratify $1 ___proof.drat $2
fi
