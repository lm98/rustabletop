#import "@rtt/std:0.1.0": outline, safe-area, icon-image

#let suits = (
  fuoco: rgb("#b5432f"),
  acqua: rgb("#2f6db5"),
  terra: rgb("#5f7a2c"),
  aria: rgb("#8a8fa3"),
)
#let gold = rgb("#c9a94a")

#let front(seal, ctx) = {
  let color = suits.at(seal.suit)
  rect(width: 100%, height: 100%, fill: color, stroke: none)
  outline(ctx, inset: 1.2mm, fill: none, stroke: 0.6pt + white)
  safe-area(ctx, inset: 0.5mm, align(center + horizon, {
    icon-image(seal.suit, height: 9mm)
    v(0.6mm, weak: true)
    text(font: "Cardo", size: 7pt, weight: "bold", fill: white, seal.name)
  }))
}

#let back(seal, ctx) = {
  rect(width: 100%, height: 100%, fill: rgb("#1f1a2e"), stroke: none)
  outline(ctx, inset: 1.5mm, fill: none, stroke: 0.8pt + gold)
  safe-area(ctx, align(center + horizon,
    text(font: "Cardo", size: 16pt, fill: gold, smallcaps[S])))
}
