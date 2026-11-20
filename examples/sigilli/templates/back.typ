#let card-size = (width: 63mm, height: 88mm)
#let bleed = 3mm

#let back() = {
  let w = card-size.width
  let h = card-size.height
  box(width: w + 2 * bleed, height: h + 2 * bleed, fill: rgb("#1f1a2e"), {
    place(center + horizon,
      rect(width: w - 8mm, height: h - 8mm, radius: 3mm, stroke: 1pt + rgb("#c9a94a")))
    place(center + horizon,
      text(font: "Cardo", size: 16pt, fill: rgb("#c9a94a"), tracking: 2pt, smallcaps[Sigilli]))
  })
}
