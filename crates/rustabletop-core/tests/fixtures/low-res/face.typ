#import "@rtt/std:0.1.0": art, fit

#let front(item, ctx) = {
  place(top + left, dx: ctx.bleed + 5mm, dy: ctx.bleed + 5mm,
    box(width: 50mm, height: 40mm, art(item.art, placeholder: [art])))
  if ctx.draft {
    place(bottom + left, dx: ctx.bleed + 5mm, dy: -(ctx.bleed + 5mm),
      box(width: 30mm, height: 4mm, fit([#item.name #item.name #item.name #item.name #item.name], max: 12pt, min: 11pt, field: "name")))
  }
}
