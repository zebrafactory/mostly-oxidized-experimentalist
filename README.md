# Mostly Oxidized Experimentalist

This shall be our Alchemy-themed entry for [Bevy Jam 8](https://itch.io/jam/bevy-jam-8).

We like [Bevy](https://bevy.org/).

We like [Rust](https://rust-lang.org/).

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --locked
cargo install wasm-opt --locked

cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir target/web/ target/wasm32-unknown-unknown/release/mostly-oxidized-experimentalist.wasm
wasm-opt -Oz -o target/web/mostly-oxidized-experimentalist_bg.wasm target/web/mostly-oxidized-experimentalist_bg.wasm
```

For iterating with native build:

```sh
sudo apt-get install pkg-config libx11-dev libasound2-dev libudev-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libwayland-dev
cargo run
```