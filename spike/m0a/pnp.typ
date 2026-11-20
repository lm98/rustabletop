#import "card.typ": front, back, card-size, bleed

#let data-path = sys.inputs.at("data", default: "cards.csv")
#let rows = csv(data-path, row-type: dictionary)
#let instances = rows.map(r => range(int(r.qty)).map(_ => r)).flatten()

#let page-w = 210mm
#let page-h = 297mm
#let min-margin = 5mm
#let cw = card-size.width
#let ch = card-size.height

#let cols = int((page-w - 2 * min-margin - 2 * bleed) / cw)
#let rows-per-page = int((page-h - 2 * min-margin - 2 * bleed) / ch)
#let per-page = cols * rows-per-page
#let grid-w = cols * cw
#let grid-h = rows-per-page * ch
#let x0 = (page-w - grid-w) / 2
#let y0 = (page-h - grid-h) / 2

#let crop-marks() = {
  let len = 4mm
  let gap = bleed + 1mm
  let stroke = 0.25pt + black
  for i in range(cols + 1) {
    let x = x0 + i * cw
    place(top + left, line(start: (x, y0 - gap - len), end: (x, y0 - gap), stroke: stroke))
    place(top + left, line(start: (x, y0 + grid-h + gap), end: (x, y0 + grid-h + gap + len), stroke: stroke))
  }
  for j in range(rows-per-page + 1) {
    let y = y0 + j * ch
    place(top + left, line(start: (x0 - gap - len, y), end: (x0 - gap, y), stroke: stroke))
    place(top + left, line(start: (x0 + grid-w + gap, y), end: (x0 + grid-w + gap + len, y), stroke: stroke))
  }
}

#let cell(content, col, row, occupied) = {
  let free(c, r) = not occupied.contains((c, r))
  let l = if free(col - 1, row) { bleed } else { 0mm }
  let r = if free(col + 1, row) { bleed } else { 0mm }
  let t = if free(col, row - 1) { bleed } else { 0mm }
  let b = if free(col, row + 1) { bleed } else { 0mm }
  place(top + left, dx: x0 + col * cw - l, dy: y0 + row * ch - t,
    box(width: cw + l + r, height: ch + t + b, clip: true,
      place(top + left, dx: -(bleed - l), dy: -(bleed - t), content)))
}

#let sheet(items, render, mirror: false) = {
  crop-marks()
  let slot(i) = {
    let col = calc.rem(i, cols)
    (if mirror { cols - 1 - col } else { col }, calc.div-euclid(i, cols))
  }
  let occupied = range(items.len()).map(slot)
  for (i, item) in items.enumerate() {
    let (col, row) = slot(i)
    cell(render(item), col, row, occupied)
  }
}

#set page(width: page-w, height: page-h, margin: 0pt)

#let chunks = instances.chunks(per-page)
#for (n, chunk) in chunks.enumerate() {
  sheet(chunk, front)
  pagebreak()
  sheet(chunk, _ => back(), mirror: true)
  if n < chunks.len() - 1 { pagebreak() }
}
