# Diagnostic codes

Every diagnostic has a stable `code`, a `severity` (`error` blocks the build, `warning` does
not but must be fixed before delivering), a `location` (file, line, column) and a `subject`
(`component / instance / field`). Hints are usually the exact fix.

## Data (`data/*.yaml`, `fields`)

| Code | Meaning | Typical fix |
|---|---|---|
| `data.required` | A field listed in `required` is missing. | Add the field to the record, or drop it from `required`. |
| `data.unknown-field` | The record has a field not declared in `schema.properties` (often a typo). | Fix the name (see `did you mean`) or declare the field. |
| `data.type` | Wrong type: `qty: three`, `value: "5"`. | Write the value with the declared type; quote strings that look like numbers or booleans only if the type is string. |
| `data.enum` | Value not in `enum`. | Use one of the allowed values listed in the hint. |
| `data.id` | `id` is not a lowercase slug. | `fire-mage`, not `Fire Mage` or `fire_mage`. |
| `data.duplicate-id` | Two records share an `id`. | Rename one, or merge them and use `qty`. |
| `data.invalid` | Another schema constraint failed (`maxLength`, `minimum`, `pattern`, …). | Read the message; shorten or change the value, or relax the schema. |
| `data.shape` | The data file is not a list of mappings. | Start each record with `- id: ...`. |
| `data.unknown-icon` | `:name:` in a `rich-text` field is not a declared icon. | Fix the name or declare the icon under `icons:`. |
| `data.missing-asset` | A `format: asset` file does not exist. | Fix the path (relative to the project root) or add the file; or remove the field and let `art` show its placeholder. |
| `data.grid-cell` | A `format: grid-cell` value is not `[col, row]` inside the grid. | The hint gives the bounds: columns and rows start at 0. |

## Manifest (`game.yaml`)

| Code | Meaning | Typical fix |
|---|---|---|
| `manifest.missing` | No `game.yaml` in the directory. | Run the command on the project directory, or `rustabletop new`. |
| `manifest.invalid` | YAML structure does not match the manifest: unknown key, wrong type. Stops at the first one. | See `reference/manifest.md` for the exact keys. |
| `manifest.id` | Component id is not a lowercase slug. | `unit-card`. |
| `manifest.duplicate-id` | Two components share an id. | Rename one. |
| `manifest.shape` | Shape not available for the kind, or `orientation` / `radius` on the wrong shape. | Tokens: `circle`, `square`, `hex`; cards and boards: `rect`. |
| `manifest.size` | Size missing, unknown preset, wrong form for the shape. | `rect`: preset or `{ width, height }`; others: one length. |
| `manifest.face` | Unknown face name or missing `front`. | Faces are `front` and `back`. |
| `manifest.face-function` | The template file does not define the face function. | Export `#let front(item, ctx) = { ... }` (name = face). |
| `manifest.missing-file` | A face template, icon, data file or font directory does not exist. | Fix the path, relative to the project root. |
| `manifest.icon` | Bad icon name or not an SVG/PNG. | Lowercase name with dashes; `.svg` or `.png`. |
| `manifest.schema` | The schema uses an unsupported keyword or type, redefines `id`/`qty`, or uses `grid-cell` without a `grid`. | Use only the keywords listed in `reference/manifest.md`. |
| `manifest.data` | Both `data` and `fields`, or `fields` is not a mapping. | Keep one. |
| `manifest.data-format` | Data file is not `.yaml`, `.yml` or `.csv`. | Use YAML. |
| `manifest.print` | A `print` option not available for the kind, or an invalid combination (`sides: duplex` without `back`, `back` on `tiles`, `overlap` > 50mm). | See the per-kind table in `reference/manifest.md`. |
| `manifest.grid` | `grid` on a non-board, `cell` neither a length nor a suitable component, orientation conflict, grid larger than the board. | Shrink `cols`/`rows`, change `origin`, or use a `hex` token for a hex grid. |

## Files

| Code | Meaning | Typical fix |
|---|---|---|
| `yaml.syntax` | YAML syntax error. | Usually indentation, or a string with `: ` or starting with `:`, `*`, `&`, `[` … that needs quotes. Quote texts containing icons: `text: ':sun: Draw a card.'`. |
| `yaml.multiple-documents` | The file has several `---` documents. | Keep one document. |
| `yaml.unsupported` | Aliases (`*ref`), anchors merges or complex keys. | Write the values out. |
| `csv.syntax` | Malformed CSV, typically an unquoted comma. | Quote the field, or switch to YAML. |

## Build and render

| Code | Meaning | Typical fix |
|---|---|---|
| `render.overflow` | A `fit` text does not fit at its `min` size (warning). | Shorten the text in the data. Otherwise give it a bigger box in the template, or lower `min` only if it stays ≥ 6pt. |
| `render.low-resolution` | An `art` image prints below 200 dpi (warning). | Use an image at least as large as the hint says, or an SVG. |
| `render.target` | `rustabletop render` target not found: unknown component, instance or face, or a component with several instances given without `/<instance>`. | Use `<component>/<instance>` as listed in the hint or by `rustabletop schema`. |
| `print.too-large` | A card or token does not fit on the paper. | Smaller size, larger paper, or `kind: board`. |
| `typst` | Error or warning from the Typst template itself; the location is in the `.typ` file. | Fix the template code. Common causes: `#` inside a code block, a missing `import` from `@rtt/std`, `none` used as content or number (an optional field), a misspelled field. |
