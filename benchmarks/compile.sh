#!/bin/sh

mkdir -p builds

# Compile Rust binary in release mode
pushd rust-vp6
export RUSTFLAGS="-C target-feature=+sse4.2,+avx2"
cargo build --release --target-dir target
cp target/release/rust-vp6 ../builds
popd

# Compile C binary in optimized mode
pushd c-vp6
cc -O3 main.c -o ../builds/c-vp6 `pkg-config --cflags --libs libavutil libavcodec libavformat`
popd