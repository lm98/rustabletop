# rustabletop

Agent-first framework for turning tabletop game data into printable and digital artifacts.
Rendering is delegated to an embedded [Typst](https://typst.app); rustabletop owns schema validation,
instance expansion, imposition and export.

Status: **M2 (`rtt build`) done** — see the project doc "Decisioni v0.1" for scope and milestones.

## A project

```
examples/sigilli/
├── game.yaml          manifest: icons, print settings, components (size, bleed, JSON Schema, faces, data)
├── data/units.yaml    one record per card (`id` and `qty` are built-in fields)
├── templates/card.typ Typst file exporting `front(card, ctx)` and `back(card, ctx)`
├── icons/             SVG/PNG icons used as `:name:` in `rich-text` fields
└── fonts/             project fonts (system fonts are never used)
```

```sh
cargo run -p rtt-cli -- check examples/sigilli          # human-readable
cargo run -p rtt-cli -- check examples/sigilli --json   # for agents
cargo run -p rtt-cli -- build examples/sigilli          # out/sigilli-pnp.pdf + out/unit-card/<id>-<face>.png
```

### Face templates

A face is a Typst function named after the face. It draws on the trim area plus bleed on every side,
with the origin at the bleed corner; rustabletop sets size, clipping and pages.

```typst
#import "@rtt/std:0.1.0": rich, fit, safe-area, art

#let front(card, ctx) = {
  rect(width: 100%, height: 100%, fill: red)
  safe-area(ctx, inset: 3mm)[#fit(rich(card.text), field: "text")]
}
```

- `card`: the validated record; every declared field is present (`none` when absent). `format: asset` fields arrive as paths.
- `ctx`: `(trim: (width, height), bleed, component, face)`.
- `rich(text)` replaces `:name:` with the icons declared in the manifest without changing line spacing.
- `fit(body, max: 9pt, min: 5.5pt, field: none)` shrinks text to its container; if it does not fit at `min`,
  `rtt build` reports `render.overflow` at the record (at the field, when `field` is given).
- `safe-area(ctx, inset:)` places content inside the trim; `art(path, placeholder:)` shows an image or a placeholder.

### Printing

```yaml
print:
  paper: a4            # a4 | letter
  margin: 5mm
  crop-marks: true
  duplex: long-edge    # long-edge | short-edge (backs flipped vertically and rotated) | none (no back sheets)
```

Cards are butted at the trim, bleed is drawn only on sides without a neighbouring card,
and back sheets mirror the front sheets for duplex printing.

Diagnostics point at the data, not at Typst: file, line, component / instance / field,
plus hints (allowed values, `did you mean`). Schemas are a strict subset of JSON Schema:
unknown keywords are rejected and records reject undeclared fields by default.

## Layout

```
crates/rtt-core   manifest + data loading with source positions, schema validation, imposition, build (generated Typst main + virtual JSON data), Typst World (project-confined files, project fonts only, embedded `@rtt/std`), PDF/PNG export, diagnostics
crates/rtt-core/typst/std   the `@rtt/std` Typst package, embedded in the binary
crates/rtt-cli    `rtt` binary (`check`, `build`, low-level `render`)
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

Fonts in `spike/m0a/fonts` and `examples/sigilli/fonts` are Cardo and Fira Sans, under the SIL Open Font License (see the `OFL-*.txt` files).
