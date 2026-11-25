# Face templates

A face is a Typst function named after the face, exported by the file listed in `faces`:

```typst
#import "@rtt/std:0.1.0": rich, fit, safe-area, outline, art

#let front(item, ctx) = { ... }
#let back(item, ctx) = { ... }   // same file or another one
```

`rustabletop` calls it once per instance and face, on a canvas of **trim + 2 × bleed** with the origin
at the top-left **bleed** corner. It sets the size, clips to the canvas and places the face on
the sheets. Never use `#set page`, `#pagebreak()` or fixed card sizes.

## `item`

The validated record: `item.id`, `item.qty` and every field declared in the schema. Absent
optional fields are `none`: test with `if item.flavor != none { ... }`. `format: asset` fields
arrive as paths usable by `art()`; `grid-cell` fields as `(col, row)` arrays.

## `ctx`

| Field | Value |
|---|---|
| `ctx.kind` | `"card"`, `"token"` or `"board"` |
| `ctx.trim` | `(width, height)`: the finished size |
| `ctx.bleed` | bleed length; the trim starts at `(ctx.bleed, ctx.bleed)` |
| `ctx.shape` | `(type, orientation, width, height, radius, inscribed)`; `inscribed` is `(x, y, width, height)`, the largest rectangle inside the shape, relative to the trim corner |
| `ctx.component`, `ctx.face`, `ctx.item` | component id, face name, instance id |
| `ctx.grid` | the board's grid or `none`: `(type, orientation, cols, rows, cell, width, height, origin, centers)` |
| `ctx.components` | every component's `(kind, shape, trim, bleed)` by id: `ctx.components.card.trim.width` |
| `ctx.draft` | `true` with `--draft`: no filled backgrounds, outlines and dark text only |

## `@rtt/std`

| Function | Use |
|---|---|
| `safe-area(ctx, inset: 3mm, body)` | Places `body` in a box over `ctx.shape.inscribed` shrunk by `inset`. Put all text inside. |
| `fit(body, max: 9pt, min: 5.5pt, field: none)` | Largest text size between `max` and `min` that fits the available box. Below `min` it reports `render.overflow` on the record (on `field` if given: **always give it**). Needs a box of known height around it: `box(width:, height:)`, a grid row with fixed or `1fr` height, or `safe-area`. |
| `rich(text)` | Replaces `:name:` with declared icons inline, without changing the line height. Use on `rich-text` fields. |
| `icon(name, size: 0.84em)` | One inline icon. |
| `icon-image(name, ..args)` | An icon as a free image: `icon-image("sun", width: 10mm)`. |
| `outline(ctx, inset: 0mm, ..args)` | The shape's contour (circle, hex, rounded rect) shrunk by `inset`; `fill`, `stroke` as for `rect`. Use it for token borders. |
| `art(source, width: 100%, height: 100%, fit: "cover", fill: luma(225), placeholder: none)` | An image, or a filled placeholder showing `placeholder` when `source` is `none`. In draft mode only an outline. Put it in a sized `box`. |
| `hexagon(width, height, orientation, ..args)` | A hexagon polygon. |
| `draw-grid(grid, stroke:, fill:, inset:, cell:)` | Draws all cells of `ctx.grid`. `fill` is a colour or `(col, row) => colour`; `cell` is `(col, row) => content` centred in the cell. |
| `cells(grid)` | Array of `(col, row, x, y)`: every cell centre in canvas coordinates. |
| `cell-center(grid, col, row)` | `(x, y)` of one cell centre in canvas coordinates. |
| `cell-shape(grid, inset: 0mm, ..args)` | One cell's outline, to `place` yourself. |

Canvas coordinates include the bleed: use them directly with `place(top + left, dx: x, dy: y, …)`.

## Typst essentials

- Inside `{ ... }` you are in code: no `#`. Inside `[ ... ]` you are in markup: `#` before code.
- Positioning: `place(top + left, dx: ctx.bleed + 5mm, dy: ctx.bleed + 5mm, content)`; other
  anchors: `center + horizon`, `bottom + right` (with negative `dx`/`dy` to move inwards).
- Text: `text(size: 9pt, weight: "bold", fill: white, font: "Libertinus Serif")[...]`. Embedded
  fonts: Libertinus Serif (default), New Computer Modern, DejaVu Sans Mono. Others need `fonts/`.
- Colours: `rgb("#b5432f")`, `color.lighten(60%)`, `luma(120)`.
- Layout: `grid(columns: (1fr, auto), rows: (8mm, 32mm, 1fr), row-gutter: 2mm, a, b, c)`,
  `stack(dir: ttb, spacing: 2mm, a, b)`, `box(width: 20mm, height: 8mm, body)`, `align(center, body)`.
- Shapes: `rect(width:, height:, radius:, fill:, stroke: 0.6pt + black)`, `circle(radius:)`,
  `line(start: (0mm, 0mm), end: (10mm, 0mm))`, dashed: `stroke: (paint: black, thickness: 1pt, dash: "dashed")`.
- Numbers to text: `str(item.value)`. Dictionaries: `colors.at(item.suit)`;
  `dict.at(key, default: x)`. Loops: `for (i, name) in names.enumerate() { ... }`.
- Field names may contain dashes: `item.max-hp` works.
- Repeated decoration across faces: write a helper `#let frame(ctx, color) = { ... }` in the
  template file and call it from `front` and `back`.

## Draft mode

```typst
let (background, ink) = if ctx.draft { (none, black) } else { (rgb("#2d2a3e"), white) }
if background != none { rect(width: 100%, height: 100%, fill: background, stroke: none) }
```

Everything filled becomes an outline in draft: badges, cells, banners.

## Card pattern

```typst
#import "@rtt/std:0.1.0": rich, fit, safe-area, art, outline

#let colors = (sun: rgb("#c8842a"), moon: rgb("#4a5a8c"))

#let front(card, ctx) = {
  let color = colors.at(card.suit)
  if not ctx.draft { rect(width: 100%, height: 100%, fill: color.lighten(85%), stroke: none) }
  outline(ctx, inset: 2mm, fill: none, stroke: 0.8pt + color)
  safe-area(ctx, inset: 4mm, grid(
    columns: 100%,
    rows: (8mm, 32mm, 1fr, auto),        // title, art, text (takes the rest), footer
    row-gutter: 2.5mm,
    grid(columns: (1fr, auto), align: horizon,
      fit(text(weight: "bold", card.name), max: 11pt, min: 8pt, field: "name"),
      circle(radius: 4mm, fill: if ctx.draft { none } else { color }, stroke: 0.6pt + color,
        align(center + horizon, text(weight: "bold", fill: if ctx.draft { black } else { white }, str(card.value))))),
    art(card.art, placeholder: [illustration]),
    fit(rich(card.text), max: 9pt, min: 6.5pt, field: "text"),
    align(right, text(size: 5pt, fill: luma(120), card.id)),
  ))
}
```

## Token pattern

Tokens are circles, squares or hexes: draw the background over the whole canvas, the border
with `outline`, the content with `safe-area` (the inscribed rectangle is small: keep 1-3 words
or a number and an icon).

```typst
#let front(token, ctx) = {
  if not ctx.draft { rect(width: 100%, height: 100%, fill: rgb("#b5432f"), stroke: none) }
  outline(ctx, inset: 1.5mm, fill: none, stroke: 0.8pt + if ctx.draft { black } else { white })
  safe-area(ctx, inset: 0.5mm, align(center + horizon,
    text(weight: "bold", fill: if ctx.draft { black } else { white },
      fit(align(center, token.label), max: 12pt, min: 6pt, field: "label"))))
}
```

A double-sided coin or health token: `back` in the same file, `print: { sides: fold }` (or
`duplex`). A value can go on both faces (`str(token.value)`).

## Board pattern

Paint terrain from the data, label cells, and size other areas from `ctx.components`:

```typst
#import "@rtt/std:0.1.0": draw-grid, cell-center, fit

#let terrain = (plain: rgb("#e8dfc8"), forest: rgb("#7a9a5a"), water: rgb("#6a9ac8"))

#let front(board, ctx) = {
  let painted = (:)
  for space in board.spaces { painted.insert(repr(space.at), space.terrain) }
  let kind(col, row) = painted.at(repr((col, row)), default: "plain")

  if not ctx.draft { rect(width: 100%, height: 100%, fill: rgb("#d4c9ad"), stroke: none) }
  draw-grid(ctx.grid, stroke: 0.6pt + black,
    fill: (col, row) => if ctx.draft { none } else { terrain.at(kind(col, row)) },
    cell: (col, row) => text(size: 6pt, str(col) + "," + str(row)))

  let (x, y) = cell-center(ctx.grid, 0, 0)        // anything anchored to a cell
  place(top + left, dx: x - 3mm, dy: y - 3mm, circle(radius: 3mm, stroke: red))

  let deck = ctx.components.card.trim             // a slot exactly the size of a card
  place(top + right, dx: -(ctx.bleed + 20mm), dy: ctx.bleed + 70mm,
    rect(width: deck.width, height: deck.height, radius: 3mm, stroke: (paint: black, dash: "dashed")))
}
```

A track (turn, score, health) without a grid: a row of boxes computed from the trim.

```typst
let (n, size, gap) = (10, 18mm, 2mm)
let x0 = ctx.bleed + (ctx.trim.width - n * size - (n - 1) * gap) / 2
for i in range(n) {
  place(top + left, dx: x0 + i * (size + gap), dy: ctx.bleed + 20mm,
    rect(width: size, height: size, radius: 2mm, stroke: 0.8pt + black,
      align(center + horizon, text(size: 12pt, weight: "bold", str(i + 1)))))
}
```

Put the board title in a sized box with `fit(..., field: "title")`. Keep text and important
marks at least 10mm inside the trim: the outer edge is cut by hand.
