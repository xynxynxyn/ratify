#!/bin/bash

glucose $1 -certified -certified-output=___proof.drat 1> /dev/null
echo "drat-trim"
time drat-trim $1 ___proof.drat -u -f
echo -e "\nratify"
if [ -z $2 ]
then
        time cargo run --release -- $1 ___proof.drat -p -r
else
        time cargo run --release $1 ___proof.drat $2
fi
