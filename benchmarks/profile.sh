#!/bin/sh

## declare an array variable
declare -a langs=("c" "rust")

export PATH=$PATH:~/.cargo/bin

## now loop through the above array
for i in "${langs[@]}"
do
   flamegraph -o flamegraph_$i.svg builds/$i-vp6 ../data/test_vp6.avi
done

