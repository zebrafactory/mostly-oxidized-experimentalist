#!/bin/sh -ex

#cargo clean
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir target/web/ target/wasm32-unknown-unknown/release/mostly-oxidized-experimentalist.wasm
wasm-opt -Oz -o target/web/mostly-oxidized-experimentalist_bg.wasm target/web/mostly-oxidized-experimentalist_bg.wasm
cp index.html target/web/
cp -r assets/ target/web/
