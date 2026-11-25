#import "@rtt/std:0.1.0": draw-grid

#let terrain = (
  floor: rgb("#d9cfb8"),
  lava: rgb("#c2532f"),
  wall: rgb("#4a4458"),
)
#let gold = rgb("#c9a94a")

#let front(map, ctx) = {
  let painted = (:)
  for space in map.spaces {
    let (col, row) = space.at
    painted.insert(str(col) + "," + str(row), space.terrain)
  }
  let kind(col, row) = painted.at(str(col) + "," + str(row), default: "floor")
  let accent = if ctx.draft { black } else { gold }

  if ctx.draft {
    draw-grid(ctx.grid, stroke: 0.6pt + black,
      cell: (col, row) => if kind(col, row) != "floor" { text(size: 7pt, kind(col, row)) })
  } else {
    rect(width: 100%, height: 100%, fill: rgb("#2b2433"), stroke: none)
    draw-grid(ctx.grid, stroke: 0.8pt + rgb("#6b6152"), fill: (col, row) => terrain.at(kind(col, row)))
  }

  let deck = ctx.components.unit-card.trim
  place(bottom + right, dx: -(ctx.bleed + 12mm), dy: -(ctx.bleed + 10mm),
    rect(width: deck.height, height: deck.width, radius: 3mm,
      stroke: (paint: accent, thickness: 1pt, dash: "dashed"),
      align(center + horizon, text(font: "Cardo", size: 14pt, fill: accent, smallcaps[Mazzo]))))
  place(bottom + left, dx: ctx.bleed + 15mm, dy: -(ctx.bleed + 30mm),
    text(font: "Cardo", size: 36pt, fill: accent, smallcaps(map.title)))
}
