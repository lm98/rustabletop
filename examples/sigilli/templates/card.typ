#import "@rtt/std:0.1.0": rich, fit, safe-area, art

#let suits = (
  fuoco: rgb("#b5432f"),
  acqua: rgb("#2f6db5"),
  terra: rgb("#5f7a2c"),
  aria: rgb("#8a8fa3"),
)
#let ink = rgb("#2a2118")
#let parchment = rgb("#f4ecd8")
#let gold = rgb("#c9a94a")

#let front(card, ctx) = {
  let color = suits.at(card.suit)
  let inner = ctx.trim.width - 8mm
  if not ctx.draft {
    rect(width: 100%, height: 100%, fill: color.lighten(10%), stroke: none)
  }
  place(top + left, dx: ctx.bleed + 2mm, dy: ctx.bleed + 2mm,
    rect(width: ctx.trim.width - 4mm, height: ctx.trim.height - 4mm, radius: 2mm,
      fill: if ctx.draft { none } else { parchment }, stroke: 0.6pt + color.darken(30%)))

  safe-area(ctx, inset: 4mm, {
    set text(font: "Cardo", fill: ink)
    grid(columns: (1fr, auto), align: (left + horizon, right + horizon),
      text(size: 10pt, weight: "bold", card.name),
      circle(radius: 3.2mm, fill: if ctx.draft { none } else { color }, stroke: 0.5pt + if ctx.draft { color } else { white },
        align(center + horizon, text(size: 10pt, weight: "bold", fill: if ctx.draft { ink } else { white }, str(card.cost)))),
    )

    place(top + left, dy: 10mm,
      box(width: inner, height: 34mm, radius: 1mm, clip: true,
        art(card.art, fill: color.lighten(55%), placeholder: text(font: "Fira Sans", fill: color.darken(20%))[illustrazione])))

    let flavor-h = if card.flavor != none { 8mm } else { 0mm }
    place(top + left, dy: 47mm,
      box(width: inner, height: ctx.trim.height - 55mm - flavor-h, {
        set par(leading: 0.45em)
        fit(rich(card.text), field: "text")
      }))
    if card.flavor != none {
      place(bottom + left,
        block(width: inner, text(size: 6.5pt, style: "italic", fill: rgb("#5a4a3a"), card.flavor)))
    }
    place(bottom + right, dy: 1.4mm,
      text(font: "Fira Sans", size: 4.5pt, fill: color.darken(40%), card.id))
  })
}

#let back(card, ctx) = {
  let accent = if ctx.draft { ink } else { gold }
  if not ctx.draft {
    rect(width: 100%, height: 100%, fill: rgb("#1f1a2e"), stroke: none)
  }
  place(center + horizon,
    rect(width: ctx.trim.width - 8mm, height: ctx.trim.height - 8mm, radius: 3mm, stroke: 1pt + accent))
  place(center + horizon,
    text(font: "Cardo", size: 16pt, fill: accent, tracking: 2pt, smallcaps[Sigilli]))
}
