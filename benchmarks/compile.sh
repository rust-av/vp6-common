#!/bin/sh

mkdir -p builds

# Compile C binary in optimized mode
pushd c-vp6
cc -O3 main.c -o ../builds/c-vp6 `pkg-config --cflags --libs libavutil libavcodec libavformat`
popd