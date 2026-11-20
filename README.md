# rustabletop

Agent-first framework for turning tabletop game data into printable and digital artifacts.
Rendering is delegated to an embedded [Typst](https://typst.app); rustabletop owns schema validation,
instance expansion, imposition and export.

Status: **M1 (`rtt check`) done** — see the project doc "Decisioni v0.1" for scope and milestones.

## A project

```
examples/sigilli/
├── game.yaml          manifest: components, size, bleed, JSON Schema, faces, data source
├── data/units.yaml    one record per card (`id` and `qty` are built-in fields)
└── templates/         Typst face templates (contract defined in M2)
```

```sh
cargo run -p rtt-cli -- check examples/sigilli          # human-readable
cargo run -p rtt-cli -- check examples/sigilli --json   # for agents
```

Diagnostics point at the data, not at Typst: file, line, component / instance / field,
plus hints (allowed values, `did you mean`). Schemas are a strict subset of JSON Schema:
unknown keywords are rejected and records reject undeclared fields by default.

## Layout

```
crates/rtt-core   manifest + data loading with source positions, schema validation, Typst World (project-confined files, project fonts only), compile, PDF/PNG export, diagnostics
crates/rtt-cli    `rtt` binary (`check`, low-level `render`)
examples/sigilli  reference project
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
