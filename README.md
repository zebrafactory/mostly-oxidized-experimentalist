# Mostly Oxidized Experimentalist

This shall be our Alchemy-themed entry for [Bevy Jam 8](https://itch.io/jam/bevy-jam-8).

We like [Bevy](https://bevy.org/).

We like [Rust](https://rust-lang.org/).

```sh
rustup target install wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo build --release --target wasm32-unknown-unknown
wasm-server-runner target/wasm32-unknown-unknown/release/mostly-oxidized-experimentalist.wasm
```