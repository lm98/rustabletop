#import "card.typ": front, back, card-size, bleed

#let data-path = sys.inputs.at("data", default: "cards.csv")
#let rows = csv(data-path, row-type: dictionary)
#let only = sys.inputs.at("id", default: none)
#let selected = if only == none { rows } else { rows.filter(r => r.id == only) }

#set page(
  width: card-size.width + 2 * bleed,
  height: card-size.height + 2 * bleed,
  margin: 0pt,
)

#for (i, r) in selected.enumerate() {
  front(r)
  if i < selected.len() - 1 { pagebreak() }
}
