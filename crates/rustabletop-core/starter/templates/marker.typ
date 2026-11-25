#import "@rtt/std:0.1.0": outline, safe-area, fit

#let players = (
  red: rgb("#b5432f"),
  blue: rgb("#2f6db5"),
)

#let side(marker, ctx, body) = {
  let color = players.at(marker.player)
  let (background, foreground) = if ctx.draft { (none, color) } else { (color, white) }
  if background != none {
    rect(width: 100%, height: 100%, fill: background, stroke: none)
  }
  outline(ctx, inset: 1.5mm, fill: none, stroke: 0.8pt + foreground)
  safe-area(ctx, inset: 0.5mm, align(center + horizon, text(fill: foreground, weight: "bold", body)))
}

#let front(marker, ctx) = side(marker, ctx,
  fit(align(center, marker.label), max: 10pt, min: 6pt, field: "label"))

#let back(marker, ctx) = side(marker, ctx, text(size: 14pt, upper(marker.player.first())))
