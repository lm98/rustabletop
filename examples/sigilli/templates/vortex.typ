#import "@rtt/std:0.1.0": outline, safe-area

#let front(vortex, ctx) = {
  if not ctx.draft {
    rect(width: 100%, height: 100%, fill: rgb("#3b2a5c"), stroke: none)
  }
  outline(ctx, inset: 1.5mm, fill: if ctx.draft { none } else { rgb("#4b3a70") },
    stroke: 0.8pt + if ctx.draft { rgb("#3b2a5c") } else { rgb("#c9a94a") })
  safe-area(ctx, inset: 1mm, align(center + horizon,
    text(font: "Cardo", size: 10pt, weight: "bold", fill: if ctx.draft { rgb("#3b2a5c") } else { rgb("#f4ecd8") },
      smallcaps(vortex.label))))
}
