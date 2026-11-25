#import "@rtt/std:0.1.0": rich, fit, safe-area, art, outline

#let suits = (
  sun: rgb("#c8842a"),
  moon: rgb("#4a5a8c"),
)
#let ink = rgb("#222222")

#let front(card, ctx) = {
  let color = suits.at(card.suit)
  if not ctx.draft {
    rect(width: 100%, height: 100%, fill: color.lighten(85%), stroke: none)
  }
  outline(ctx, inset: 2mm, fill: none, stroke: 0.8pt + color)

  safe-area(ctx, inset: 4mm, {
    set text(fill: ink)
    grid(
      columns: 100%,
      rows: (8mm, 32mm, 1fr, auto),
      row-gutter: 2.5mm,
      grid(
        columns: (1fr, auto),
        align: (left + horizon, right + horizon),
        fit(text(weight: "bold", card.name), max: 11pt, min: 8pt, field: "name"),
        circle(radius: 4mm, fill: if ctx.draft { none } else { color }, stroke: 0.6pt + color,
          align(center + horizon, text(size: 10pt, weight: "bold", fill: if ctx.draft { ink } else { white }, str(card.value)))),
      ),
      art(card.art, fill: color.lighten(60%), placeholder: [illustration]),
      fit(rich(card.text), max: 9pt, min: 6.5pt, field: "text"),
      align(right, text(size: 5pt, fill: luma(120), card.id)),
    )
  })
}

#let back(card, ctx) = {
  let (background, foreground) = if ctx.draft { (none, ink) } else { (rgb("#2d2a3e"), rgb("#e0c068")) }
  if background != none {
    rect(width: 100%, height: 100%, fill: background, stroke: none)
  }
  outline(ctx, inset: 4mm, fill: none, stroke: 1pt + foreground)
  place(center + horizon, text(size: 16pt, weight: "bold", fill: foreground, tracking: 2pt, upper(ctx.component)))
}
