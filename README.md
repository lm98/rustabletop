# rustabletop

Agent-first framework for turning tabletop game data into printable and digital artifacts.
Rendering is delegated to an embedded [Typst](https://typst.app); rustabletop owns schema validation,
instance expansion, imposition and export.

Status: **M2.6 (`board`, grids, tile assembly) done** — see the project doc "Decisioni v0.1" for scope and milestones.

## A project

```
examples/sigilli/
├── game.yaml          manifest: icons, print settings, components (size, bleed, JSON Schema, faces, data)
├── data/              one record per item (`id` and `qty` are built-in fields)
├── templates/         Typst files exporting one function per face: `front(item, ctx)`, `back(item, ctx)`
├── icons/             SVG/PNG icons used as `:name:` in `rich-text` fields
└── fonts/             project fonts (system fonts are never used)
```

```sh
cargo run -p rtt-cli -- check examples/sigilli          # human-readable
cargo run -p rtt-cli -- check examples/sigilli --json   # for agents
cargo run -p rtt-cli -- build examples/sigilli          # out/sigilli-pnp.pdf + out/<component>/<id>-<face>.png
```

### Components

A component is a shape, a set of faces and a way to produce it; `kind` sets the defaults.

| kind | shapes (`size`) | faces | `print` options (default first) |
|---|---|---|---|
| `card` | `rect`: preset or `{width, height}`, optional `radius` | `front`, [`back`] | `duplex`, `single` + `crop-marks` |
| `token` | `circle` (diameter), `square` (side, optional `radius`), `hex` (across flats, `orientation: pointy \| flat`) | `front`, [`back`] | `sides`: `fold`, `duplex`, `single` + `cut-lines`, `gap` |
| `board` | `rect`: `{width, height}`, optional `radius` | `front`, [`back`] (`poster` only) | `mode`: `tiles`, `poster` + `overlap` (default 10mm) |

```yaml
- id: seal
  kind: token
  shape: circle
  size: 25mm
  bleed: 2mm
  faces: { front: templates/seal.typ, back: templates/seal.typ }
  data: data/seals.yaml
  print: { sides: fold }     # fronts on the left half, mirrored backs on the right: fold, glue, cut
```

Without `back` a component prints single-sided. `data` is optional: without it the component has one
instance whose id is the component id, with the fields given inline in `fields` (validated by `schema`,
which is optional too).

### Face templates

A face is a Typst function named after the face. It draws on the shape's bounding box plus bleed on
every side, with the origin at the bleed corner; rustabletop sets size, clipping (rectangular) and pages.
Backgrounds should overflow the contour, as for cards.

```typst
#import "@rtt/std:0.1.0": rich, fit, safe-area, outline, art

#let front(item, ctx) = {
  rect(width: 100%, height: 100%, fill: red)
  outline(ctx, inset: 1mm, fill: none, stroke: white)
  safe-area(ctx, inset: 1mm)[#fit(rich(item.text), field: "text")]
}
```

- `item`: the validated record; every declared field is present (`none` when absent). `format: asset` fields arrive as paths.
- `ctx`: `(kind, shape: (type, orientation, width, height, radius, inscribed), trim: (width, height), bleed, component, face, item, grid, components)`.
  `ctx.components` maps every component id to its `(kind, shape, trim, bleed)`, e.g. `ctx.components.unit-card.trim`.
- `rich(text)` replaces `:name:` with the icons declared in the manifest without changing line spacing.
- `fit(body, max: 9pt, min: 5.5pt, field: none)` shrinks text to its container; if it does not fit at `min`,
  `rtt build` reports `render.overflow` at the record (at the field, when `field` is given).
- `safe-area(ctx, inset:)` places content in the largest rectangle inscribed in the shape, shrunk by `inset`.
- `outline(ctx, inset:, ..args)` draws the shape's contour shrunk by `inset` (`fill`, `stroke` as for `rect`).
- `icon-image(name, ..args)` is a declared icon as a free-standing image; `art(path, placeholder:)` shows an image or a placeholder.
- `draw-grid(grid, stroke:, fill:, inset:, cell:)` draws `ctx.grid`; `fill` and `cell` may be functions of `(col, row)`.
  `cell-center(grid, col, row)` and `cells(grid)` give centres in canvas coordinates (bleed included); `cell-shape(grid, ..args)` is one cell's outline.

### Boards and grids

```yaml
- id: dungeon-map
  kind: board
  size: { width: 500mm, height: 350mm }
  faces: { front: templates/map.typ }
  fields: { title: Le Cripte, spaces: [{ at: [3, 4], terrain: lava }] }
  schema:
    properties:
      title: { type: string }
      spaces: { type: array, items: { properties: { at: { format: grid-cell }, terrain: { enum: [floor, lava] } } } }
  grid: { type: hex, cols: 14, rows: 9, cell: vortex, origin: [25mm, 40mm] }
  print: { mode: tiles, overlap: 10mm }
```

- `grid.cell` is a length (hex: across flats; square: side) or a component id: a `hex` grid takes the size
  and orientation of a `hex` token, a `square` grid the side of a `square` token or the diameter of a `circle`.
  `origin` is the centre of cell `(0, 0)` from the trim corner; by default the grid is centred.
- Cells use offset coordinates `[col, row]` from the top left: odd rows shifted for `pointy` hexes (odd-r),
  odd columns for `flat` ones (odd-q). `format: grid-cell` values are checked against the grid (`data.grid-cell`).
- `tiles` cuts the face into page-sized sections (A1, A2, … rows by letter) that overlap their right and lower
  neighbours. Each section marks the strip that goes under its neighbour, the edges to trim (✂) and the
  registration marks; an assembly page with a thumbnail comes first. `poster` prints one page of board + bleed.

### Printing

```yaml
print:                 # project: the printer and its paper
  paper: a4            # a4 | letter
  margin: 5mm
  duplex-edge: long    # long | short (backs flipped vertically and rotated)
```

Cards are butted at the trim with crop marks; bleed is drawn only on sides without a neighbouring card.
Tokens are spaced by `gap` (default 2mm), bleed on every side, with a cut line on the contour
(white under black, visible on any background).
Duplex back sheets mirror the front sheets according to `duplex-edge`.

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
