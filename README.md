# rustabletop

Agent-first framework for turning tabletop game data into printable and digital artifacts.
Rendering is delegated to an embedded [Typst](https://typst.app); rustabletop owns schema validation,
instance expansion, imposition and export.

Status: **M0 (spikes) done** — see the project doc "Decisioni v0.1".

## Layout

```
crates/rtt-core   Typst World (project-confined files, project fonts only), compile, PDF/PNG export, diagnostics
crates/rtt-cli    `rtt` binary
spike/m0a         pure-Typst print-and-play spike: card.typ (faces), pnp.typ (imposition), cards.typ (one card per page)
```

## Try the spike

```sh
cargo build --release
cd spike/m0a
../../target/release/rtt render pnp.typ --font-path fonts --out out
../../target/release/rtt render cards.typ -f png --font-path fonts --out out --input id=ash-titan
../../target/release/rtt render pnp.typ --font-path fonts --input data=cards.csv --json
```

`build.py` renders the same spike through the `typst` Python package, without Rust.

Fonts in `spike/m0a/fonts` are Cardo and Fira Sans, under the SIL Open Font License (see the `OFL-*.txt` files).
