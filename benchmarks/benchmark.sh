#!/bin/sh

RUNS=3

hyperfine -r ${RUNS} \
    -L program c,rust \
    'builds/{program}-vp6 ../data/test_vp6.avi' \
    --export-csv vp6-bench.csv \
    --export-markdown vp6-bench.md
