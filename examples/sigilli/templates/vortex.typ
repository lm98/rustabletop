#import "@rtt/std:0.1.0": outline, safe-area

#let front(vortex, ctx) = {
  rect(width: 100%, height: 100%, fill: rgb("#3b2a5c"), stroke: none)
  outline(ctx, inset: 1.5mm, fill: rgb("#4b3a70"), stroke: 0.8pt + rgb("#c9a94a"))
  safe-area(ctx, inset: 1mm, align(center + horizon,
    text(font: "Cardo", size: 10pt, weight: "bold", fill: rgb("#f4ecd8"), smallcaps(vortex.label))))
}
