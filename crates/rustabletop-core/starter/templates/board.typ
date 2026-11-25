#import "@rtt/std:0.1.0": draw-grid, fit

#let terrain = (
  plain: rgb("#e8dfc8"),
  forest: rgb("#7a9a5a"),
  water: rgb("#6a9ac8"),
)
#let ink = rgb("#2a2a2a")

#let front(board, ctx) = {
  let painted = (:)
  for space in board.spaces {
    painted.insert(repr(space.at), space.terrain)
  }
  let kind(col, row) = painted.at(repr((col, row)), default: "plain")

  if not ctx.draft {
    rect(width: 100%, height: 100%, fill: rgb("#d4c9ad"), stroke: none)
  }
  draw-grid(ctx.grid,
    stroke: 0.6pt + ink,
    fill: (col, row) => if ctx.draft { none } else { terrain.at(kind(col, row)) },
    cell: (col, row) => text(size: 6pt, fill: luma(90), {
      if ctx.draft and kind(col, row) != "plain" { kind(col, row); linebreak() }
      str(col) + "," + str(row)
    }),
  )

  place(top + left, dx: ctx.bleed + 15mm, dy: ctx.bleed + 15mm,
    box(width: 220mm, height: 30mm,
      fit(text(weight: "bold", fill: ink, board.title), max: 40pt, min: 20pt, field: "title")))

  let deck = ctx.components.card.trim
  place(top + right, dx: -(ctx.bleed + 22mm), dy: ctx.bleed + 70mm,
    rect(width: deck.width, height: deck.height, radius: 3mm,
      stroke: (paint: ink, thickness: 0.8pt, dash: "dashed"),
      align(center + horizon, text(size: 12pt, fill: ink)[Deck])))
}
