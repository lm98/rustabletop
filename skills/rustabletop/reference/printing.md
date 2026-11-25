# Printing

`rustabletop build` writes `out/<game>-pnp.pdf` (print-and-play sheets for every component, in
manifest order) and `out/<component>/<instance>-<face>.png` (one image per face, for review
and digital use). The PDF is meant to be printed at **100% / actual size**, never "fit to page".

## Cards

- Cards are butted at the trim on the sheet, with **crop marks** at the corners (`crop-marks`).
  Cut along the marks: one straight cut per row and column. Bleed is printed only on the sides
  without a neighbouring card.
- `sides: duplex` (default with a `back`): a front sheet followed by its back sheet, mirrored so
  that backs land behind their fronts when printed double-sided. `duplex-edge` in `print` must
  match the printer setting (`long` is the usual one).
- `sides: single`: only fronts are printed; sleeve them with any back, or print the back once.

## Tokens

- Tokens are spaced by `gap`, each with its own bleed and a **cut line** on the contour
  (white under black, visible on any background).
- `sides: fold` (default with a `back`): fronts on the left half of the sheet, backs mirrored on
  the right half. Fold the sheet along the dashed centre line, glue, then cut the tokens through
  both layers. Works on any printer.
- `sides: duplex`: backs on the next sheet, like cards. Needs accurate duplex registration.

## Boards

- `mode: tiles` (default): the board is cut into page-sized **sections** that overlap by
  `overlap` (10mm). The first page is the **assembly page**: a thumbnail with the sections
  labelled A1, A2, B1… (letter = row, number = column) and the instructions.
  Each section page shows: the hatched strip that goes **under** its right or lower neighbour,
  dashed ✂ lines on the edges to cut, registration marks to align, and crop marks for the
  outer trim. Assembly: cut each section on its ✂ lines, lay it over the hatched strip of its
  neighbour aligning the marks, glue, then trim the outer edge.
- `mode: poster`: one page as large as the board plus bleed, for a print shop or a large-format
  printer. The only mode that allows a `back`.
- Paper and orientation are chosen automatically (fewest sections). A 360×250 board on A4 is
  2×2 sections.

## Draft

`rustabletop build --draft` (and `rustabletop render --draft`) sets `ctx.draft` to `true`: templates drop
filled backgrounds, `art()` draws only the placeholder outline and never embeds images. The PDF
is `out/<game>-pnp-draft.pdf`. Use it for rules playtests and quick paper prototypes; deliver
the final build.

## What to tell the player

Name the PDF, then for each component: how many sheets, single or double-sided, and how to
cut (crop marks for cards; fold-glue-cut for `fold` tokens; the assembly page for boards).
Remind them to print at 100% and, for duplex sheets, to flip on the long edge.
