#import "icons.typ": icons

#let icon(name, size: 0.84em) = box(width: 1em, height: 0.62em,
  place(center + horizon, image(icons.at(name), height: size)))

#let icon-image(name, ..args) = image(icons.at(name), ..args)

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

#let safe-area(ctx, inset: 3mm, body) = {
  let area = ctx.shape.inscribed
  place(top + left, dx: ctx.bleed + area.x + inset, dy: ctx.bleed + area.y + inset,
    box(width: area.width - 2 * inset, height: area.height - 2 * inset, body))
}

#let hexagon(width, height, orientation, ..args) = {
  let vertices = if orientation == "flat" {
    ((0.25, 0), (0.75, 0), (1, 0.5), (0.75, 1), (0.25, 1), (0, 0.5))
  } else {
    ((0.5, 0), (1, 0.25), (1, 0.75), (0.5, 1), (0, 0.75), (0, 0.25))
  }
  polygon(..args, ..vertices.map(((x, y)) => (x * width, y * height)))
}

#let outline(ctx, inset: 0mm, ..args) = {
  let s = ctx.shape
  let ratio = if s.type == "hex" {
    let flats = if s.orientation == "flat" { s.height } else { s.width }
    (flats - 2 * inset) / flats
  } else { none }
  let (width, height) = if ratio == none {
    (s.width - 2 * inset, s.height - 2 * inset)
  } else {
    (s.width * ratio, s.height * ratio)
  }
  let radius = if s.radius == none { 0pt } else { calc.max(0pt, s.radius - inset) }
  let shape = if s.type == "circle" {
    circle(radius: width / 2, ..args)
  } else if s.type == "hex" {
    hexagon(width, height, s.orientation, ..args)
  } else {
    rect(width: width, height: height, radius: radius, ..args)
  }
  place(top + left,
    dx: ctx.bleed + (s.width - width) / 2,
    dy: ctx.bleed + (s.height - height) / 2,
    shape)
}

#let art(source, width: 100%, height: 100%, fit: "cover", fill: luma(225), placeholder: none) = {
  if source == none {
    rect(width: width, height: height, fill: fill, stroke: none,
      align(center + horizon, text(size: 7pt, fill: luma(110), placeholder)))
  } else {
    image(source, width: width, height: height, fit: fit)
  }
}
