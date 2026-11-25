---
name: rustabletop
description: Build printable tabletop games (cards, tokens, boards) with the `rustabletop` CLI. Data in YAML validated by a JSON Schema subset, faces drawn by Typst templates, print-and-play PDF with imposition. Use when asked to make, prototype or print a board/card game, game components, a print-and-play kit, or when a directory contains a rustabletop `game.yaml`.
---

# rustabletop

`rustabletop` turns game data into a print-and-play PDF. You write **data** (YAML), **schemas**
(a subset of JSON Schema) and **face templates** (Typst functions). `rustabletop` validates the data,
expands instances and quantities, lays out the sheets (cards, tokens, board sections) and
reports problems as structured diagnostics pointing at your data.

## Install

```sh
rustabletop --version || curl -fsSL https://raw.githubusercontent.com/lm98/rustabletop/main/install.sh | sh
```

`install.sh` puts `rustabletop` in `~/.local/bin` (make sure it is on `PATH`). Fallback when no
release fits the platform, ~8 minutes: `cargo install --git https://github.com/lm98/rustabletop rustabletop`.

`rustabletop new` also writes this skill into `<dir>/.claude/skills/rustabletop/`, matching the
binary's version; `rustabletop skill install --dir <project>` does the same for an existing project.

## Workflow

1. `rustabletop new <dir>` — a **working** project with one card, one token and one board. Start from
   it: change data and templates rather than writing from scratch. `--paper letter` for US.
2. `rustabletop schema <dir> --json` — what the project contains: components, shapes, print options,
   effective schemas. Run it first on an existing project.
3. Edit `game.yaml`, `data/*.yaml`, `templates/*.typ`.
4. `rustabletop check <dir> --json` — fix **every** error; warnings too unless you can say why not.
5. `rustabletop render <component>/<instance> <dir>` for each template you touched, then **look at the
   PNG** (it is printed on stdout; `--face front`, `--ppi 150` by default). One instance per
   template is enough; check the longest text and the optional fields left empty.
6. `rustabletop build <dir>` — `out/<game>-pnp.pdf` plus a PNG per face. It must end with
   `0 warning(s)`. `--draft` builds a quick ink-saving proof (`<game>-pnp-draft.pdf`).

Every command takes `--json`: `{ ok, diagnostics: [{ code, severity, message, location:
{ file, line, column }, subject: { component, instance, field }, hints }] }`. Apply the hints:
they are mechanical fixes (allowed values, `did you mean`). Codes are explained in
`reference/diagnostics.md`.

## Golden rules

- **Data in YAML**, one record per distinct item, `qty` for copies. `id` is a lowercase slug
  (`fire-mage`), unique in the file. Do not use CSV for data you write.
- **Never set the page** in a template (`#set page`, `#pagebreak`): `rustabletop` sizes the face and
  lays out the sheets. A face draws on trim + 2 × bleed with the origin at the bleed corner.
- **Sizes come from `ctx`**: `ctx.trim.width`, `ctx.shape.inscribed`, `ctx.bleed`,
  `ctx.components.<id>.trim`. No hand-written card or token dimensions in templates.
- Backgrounds fill the whole face (`rect(width: 100%, height: 100%)`) so they reach the bleed.
  Keep text inside `safe-area(ctx, inset: …)`.
- **`fit(…, field: "<name>")` around every text of variable length.** It shrinks text to its box
  and reports `render.overflow` on the record when it cannot.
- A `render.overflow` is fixed by **shortening the text** in the data (or giving the text a
  bigger box). Lower `min` only if the result is still legible at print size (≥ 6pt).
- **After every template change, render and look at the image.** Most layout bugs (overlaps,
  clipped text, wrong colours) are only visible there.
- **Honour `ctx.draft`**: when it is true, no filled backgrounds, only outlines and text, and
  keep text dark on white. `art()` already draws only an outline in draft mode.
- Declare icons in `game.yaml` and write them as `:name:` in `format: rich-text` fields.
- Use the component kinds as designed: `card` for anything in a deck, `token` for counters
  and chits (`circle`, `square`, `hex`), `board` for maps and player mats.
- Make your own design decisions (theme, colours, names, card texts, layout): the brief is
  enough. Do not stop to ask about them.

## Grid coordinates

A board `grid` uses offset coordinates `[col, row]` from the top-left cell `[0, 0]`.
`pointy` hexes shift **odd rows** right by half a cell (odd-r); `flat` hexes shift **odd
columns** down (odd-q). In a `pointy` grid, the neighbours of `[c, r]` are `[c±1, r]` and, on
even rows, `[c-1, r±1]`, `[c, r±1]`; on odd rows `[c, r±1]`, `[c+1, r±1]`. Fields with
`format: grid-cell` are checked against the grid (`data.grid-cell`).

## When to read the references

| Need | Read |
|---|---|
| Any key of `game.yaml`: kinds, shapes, sizes, schemas, grids, `print` | `reference/manifest.md` |
| Writing or fixing a template: `ctx`, `@rtt/std` functions, patterns per kind | `reference/templates.md` |
| A diagnostic code you do not understand | `reference/diagnostics.md` |
| Duplex vs fold, crop marks, board sections, draft, what to tell the player | `reference/printing.md` |

The project from `rustabletop new` is the canonical example: if this skill and the starter project
disagree, the starter project is right.

## Finishing

Report to the user: the PDF path, what it contains (sheets per component), how to print it
(see `reference/printing.md`), and the PNGs you looked at.
