# `game.yaml` reference

Every key the manifest accepts. Unknown keys are errors (`manifest.invalid`). Lengths are
strings with a unit: `mm`, `cm`, `in`, `pt` (`"2.5in"`, `63mm`).

## Project layout

```
my-game/
├── game.yaml          the manifest: icons, print settings, components
├── data/              one record per item (`id` and `qty` are built-in fields)
├── templates/         Typst files exporting one function per face: `front(item, ctx)`, `back(item, ctx)`
├── icons/             SVG/PNG icons used as `:name:` in `rich-text` fields
├── fonts/             optional project fonts (system fonts are never used)
└── out/               written by `rustabletop build` and `rustabletop render`
```

Paths in the manifest are relative to the project root; files outside it cannot be read.

## Top level

```yaml
game: my-game                # name of the game; the PDF is out/<game>-pnp.pdf
icons:                       # optional: name → SVG or PNG file, used as :name: in rich-text
  sun: icons/sun.svg
print:                       # optional: the printer
  paper: a4                  # a4 (default) | letter
  margin: 5mm                # unprintable border of the printer, default 5mm
  duplex-edge: long          # long (default) | short: how the printer flips duplex sheets
fonts: fonts                 # optional: directory of .ttf/.otf files; default `fonts/` if it exists
components: [...]            # the list below
```

| Key | Meaning |
|---|---|
| `game` | Name of the game, used for the PDF file name. |
| `icons` | Map of icon names (lowercase, dashes, start with a letter) to project files. |
| `print` | Printer settings: `paper`, `margin`, `duplex-edge`. |
| `paper` | `a4` or `letter`. |
| `margin` | Printer margin; nothing is printed closer to the paper edge. |
| `duplex-edge` | `long`: backs are mirrored left-right. `short`: backs are flipped top-bottom and rotated. |
| `fonts` | Font directory inside the project. System fonts are never used; Typst's embedded fonts (Libertinus Serif, New Computer Modern, DejaVu Sans Mono) are always available. |
| `components` | List of components. |

## Component

```yaml
- id: fire-mage              # unique, lowercase words separated by dashes
  kind: card                 # card | token | board
  shape: rect                # depends on kind, see below
  size: poker                # depends on shape, see below
  orientation: pointy        # hex only: pointy (default) | flat
  radius: 3mm                # rect and square only: corner radius
  bleed: 3mm                 # extra printed margin beyond the cut, default 3mm
  schema: { ... }            # optional JSON Schema subset for the records
  faces:                     # face name → Typst file exporting `#let <face>(item, ctx)`
    front: templates/card.typ
    back: templates/card.typ
  data: data/cards.yaml      # list of records (YAML preferred, CSV accepted)
  fields: { ... }            # instead of `data`: one inline record (boards, single components)
  print: { ... }             # production options for this kind, see below
  grid: { ... }              # boards only, see below
```

| Key | Meaning |
|---|---|
| `id` | Component id, used in `rustabletop render <id>/<instance>`, `ctx.components.<id>` and `grid.cell`. |
| `kind` | `card`, `token` or `board`: sets allowed shapes, faces and print options. |
| `shape` | `rect` (card, board: the only shape, may be omitted); `circle`, `square`, `hex` (token: required). |
| `size` | `rect`: a preset (`poker` 63×88, `tarot` 70×120, `mini-euro` 44×68, `mini-usa` 41×63) or `{ width: 360mm, height: 250mm }`. `circle`: the diameter. `square`: the side. `hex`: the distance between opposite flat sides (the width of a `pointy` hex, the height of a `flat` one). |
| `width`, `height` | Keys of a `rect` size mapping. |
| `orientation` | `hex` only. `pointy`: a vertex at the top. `flat`: a flat side at the top. |
| `radius` | `rect` and `square` only: rounded corners (at most half the side). It shapes `outline()` and token cut lines; cards are still cut straight at the crop marks (round them with a corner cutter). |
| `bleed` | Printed beyond the cut line so a slightly off cut shows no white edge. 3mm for cards and boards, 2mm for tokens is typical. |
| `schema` | Validates every record; see below. Optional. |
| `faces` | `front` is required. `back` is optional (cards, tokens, `poster` boards). |
| `data` | Path of a YAML (or CSV) file holding a list of records. |
| `fields` | One inline record (no `id` needed: it defaults to the component id). Cannot be combined with `data`. Without both, the component has one empty instance. Any kind can use either: a board may have a `data` file of several boards. |
| `print` | Production options; only those of the component's kind are accepted. |
| `grid` | A grid on a board, see below. |

### Records

Every record has two built-in fields:
- `id`: required, lowercase slug, unique in the component;
- `qty`: integer ≥ 0, default 1: how many copies are printed.

All other fields must be declared in `schema.properties` (records are strict: an undeclared
field is `data.unknown-field`). A declared field that is absent arrives in the template as `none`.

### Schema

A closed subset of JSON Schema written in YAML. Accepted keywords: `type`, `enum`, `const`,
`default`, `title`, `description`, `examples`, `format`, `minimum`, `maximum`,
`exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`, `minLength`, `maxLength`, `pattern`,
`items`, `minItems`, `maxItems`, `uniqueItems`, `properties`, `required`,
`additionalProperties`, `anyOf`, `oneOf`, `allOf`, `not`. Types: `string`, `integer`, `number`,
`boolean`, `array`, `object`, `null`. Anything else is `manifest.schema`.

```yaml
schema:
  required: [name, suit, value]
  properties:
    name: { type: string, maxLength: 24 }      # maxLength keeps titles from overflowing
    suit: { enum: [sun, moon] }
    value: { type: integer, minimum: 1, maximum: 10 }
    text: { type: string, format: rich-text }
    art: { type: string, format: asset }
```

Formats checked by `rustabletop check`:
- `rich-text`: every `:name:` must be a declared icon;
- `asset`: a file in the project (PNG/JPEG ≥ 200 dpi at print size, or SVG);
- `grid-cell`: a `[col, row]` pair inside the component's `grid`.

### `print` per kind

| Kind | Option | Values (default first) |
|---|---|---|
| card | `sides` | `duplex` if there is a `back`, else `single`; `single` prints fronts only. |
| card | `crop-marks` | `true` \| `false`: marks at the corners for cutting. |
| token | `sides` | `fold` if there is a `back` (fronts left, backs mirrored right: fold, glue, cut), `duplex`, `single`. |
| token | `cut-lines` | `true` \| `false`: contour line around each token. |
| token | `gap` | Space between tokens, default `2mm`. |
| board | `mode` | `tiles`: cut into overlapping page-sized sections to glue together; `poster`: one page the size of the board (for a print shop). |
| board | `overlap` | Overlap between `tiles` sections, default `10mm`, at most `50mm`. |

A board with a `back` must use `mode: poster`.

### `grid` (boards)

```yaml
grid: { type: hex, cols: 9, rows: 7, cell: marker, orientation: pointy, origin: [27mm, 70mm] }
```

| Key | Meaning |
|---|---|
| `type` | `hex` or `square`. |
| `cols`, `rows` | Number of columns and rows (≥ 1). |
| `cell` | A length (hex: across flats; square: side) **or a component id**: a `hex` grid takes size and orientation from a `hex` token, a `square` grid from a `square` token's side or a `circle` token's diameter. Prefer the id: cells then always match the tokens. |
| `orientation` | `hex` only; inherited from the token when `cell` is an id. |
| `origin` | Centre of cell `[0, 0]` measured from the top-left corner of the trim. Default: the grid is centred on the board. |

Cells are stored in the data as `format: grid-cell` fields, at any depth of the schema:

```yaml
- id: dungeon-map
  kind: board
  size: { width: 500mm, height: 350mm }
  faces: { front: templates/map.typ }
  fields: { title: The Crypts, spaces: [{ at: [3, 4], terrain: lava }] }
  schema:
    properties:
      title: { type: string }
      spaces:
        type: array
        items: { properties: { at: { format: grid-cell }, terrain: { enum: [floor, lava] } } }
  grid: { type: hex, cols: 14, rows: 9, cell: vortex, origin: [25mm, 40mm] }   # `vortex`: a hex token
  print: { mode: tiles, overlap: 10mm }
```

A wrong cell is reported at its path in the record (`spaces.0.at`).

The grid must fit inside the board (`manifest.grid`). Pointy hex: the grid is
`cols × cell + cell/2` wide and `(rows − 1) × cell × 0.866 + cell × 1.155` tall.
Flat hex: swap the roles of columns and rows.
