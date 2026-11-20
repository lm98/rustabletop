#let card-size = (width: 63mm, height: 88mm)
#let bleed = 3mm
#let safe = 4mm

#let suits = (
  fuoco: (color: rgb("#b5432f"), glyph: "▲"),
  acqua: (color: rgb("#2f6db5"), glyph: "●"),
  terra: (color: rgb("#5f7a2c"), glyph: "■"),
  aria:  (color: rgb("#8a8fa3"), glyph: "◆"),
)

#let icon(suit) = box(width: 0.9em, height: 0.62em,
  place(center + horizon,
    circle(radius: 0.42em, fill: suits.at(suit).color, inset: 0pt,
      align(center + horizon, text(fill: white, size: 0.55em, suits.at(suit).glyph)))),
)

#let with-icons(body) = {
  show regex(":(fuoco|acqua|terra|aria):"): it => icon(it.text.trim(":"))
  body
}

#let fit-text(width, height, max: 9pt, min: 5.5pt, step: 0.25pt, body) = context {
  let size = max
  while size > min {
    let h = measure(block(width: width, text(size: size, body))).height
    if h <= height { break }
    size -= step
  }
  text(size: size, body)
}

#let front(row) = {
  let s = suits.at(row.suit)
  let w = card-size.width
  let h = card-size.height
  let inner-w = w - 2 * safe
  box(width: w + 2 * bleed, height: h + 2 * bleed, fill: s.color.lighten(10%), {
    place(top + left, dx: bleed + 2mm, dy: bleed + 2mm,
      rect(width: w - 4mm, height: h - 4mm, radius: 2mm, fill: rgb("#f4ecd8"), stroke: 0.6pt + s.color.darken(30%)))

    place(top + left, dx: bleed + safe, dy: bleed + safe,
      box(width: inner-w, {
        set text(font: "Cardo", fill: rgb("#2a2118"))
        grid(columns: (1fr, auto), align: (left + horizon, right + horizon),
          text(size: 10pt, weight: "bold", row.name),
          circle(radius: 3.2mm, fill: s.color, stroke: 0.5pt + white,
            align(center + horizon, text(size: 10pt, weight: "bold", fill: white, row.cost))),
        )
      }))

    place(top + left, dx: bleed + safe, dy: bleed + 14mm,
      rect(width: inner-w, height: 34mm, radius: 1mm, fill: s.color.lighten(55%), stroke: none,
        align(center + horizon, text(font: "Fira Sans", size: 7pt, fill: s.color.darken(20%), [illustrazione]))))

    place(top + left, dx: bleed + safe, dy: bleed + 51mm,
      box(width: inner-w, height: h - 51mm - safe, {
        set text(font: "Cardo")
        set par(leading: 0.45em)
        let flavor-h = if row.flavor != "" { 8mm } else { 0mm }
        fit-text(inner-w, h - 51mm - safe - flavor-h, with-icons(row.text))
        if row.flavor != "" {
          place(bottom + left,
            block(width: inner-w, text(size: 6.5pt, style: "italic", fill: rgb("#5a4a3a"), row.flavor)))
        }
      }))

    place(bottom + right, dx: -(bleed + safe), dy: -(bleed + 2.6mm),
      text(font: "Fira Sans", size: 4.5pt, fill: s.color.darken(40%), row.id))
  })
}

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
