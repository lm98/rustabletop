#import "icons.typ": icons

#let icon(name, size: 0.84em) = box(width: 1em, height: 0.62em,
  place(center + horizon, image(icons.at(name), height: size)))

#let rich(body) = {
  if type(body) != str { return body }
  show regex(":[a-z][a-z0-9-]*:"): it => {
    let name = it.text.slice(1, -1)
    if name in icons { icon(name) } else { it }
  }
  body
}

#let fit(body, max: 9pt, min: 5.5pt, step: 0.25pt, field: none) = layout(area => {
  let height(size) = measure(block(width: area.width, text(size: size, body))).height
  let size = max
  while size > min and height(size) > area.height { size -= step }
  if size < min { size = min }
  if height(size) > area.height {
    place([#metadata((field: field, size: size.pt())) <rtt-overflow>])
  }
  text(size: size, body)
})

#let safe-area(ctx, inset: 3mm, body) = place(top + left,
  dx: ctx.bleed + inset, dy: ctx.bleed + inset,
  box(width: ctx.trim.width - 2 * inset, height: ctx.trim.height - 2 * inset, body))

#let art(source, width: 100%, height: 100%, fit: "cover", fill: luma(225), placeholder: none) = {
  if source == none {
    rect(width: width, height: height, fill: fill, stroke: none,
      align(center + horizon, text(size: 7pt, fill: luma(110), placeholder)))
  } else {
    image(source, width: width, height: height, fit: fit)
  }
}
