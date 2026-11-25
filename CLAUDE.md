# rustabletop

Agent-first framework that turns tabletop game data (YAML/CSV + JSON Schema) into
printable and digital artifacts. Rendering is delegated to an embedded Typst;
rustabletop owns validation, instance expansion, imposition and export.

The first user is an LLM agent: every feature is judged by how well an agent can
use it through `rustabletop ... --json` without human fixes.

## Design docs

Scope, decisions and milestone specs live in the `docs/` directory in this repo:
- `Decisioni v0.1.md` — scope, out-of-scope list (with re-entry conditions), milestones
- `Spec M<n> — ….md` — the spec for the milestone being implemented

Read the current milestone spec before starting. When a milestone lands, update its
status in `Decisioni v0.1.md`. Do not expand scope beyond the spec: propose instead.

## Layout

```
crates/rustabletop-core   manifest, data loading with source positions, schema, Typst World, diagnostics
crates/rustabletop    the `rustabletop` binary
examples/sigilli  reference project — must always pass `rustabletop check` (and `rustabletop build` from M2)
spike/m0a         frozen M0 spike, reference only; do not extend
```

## Commands

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -p rustabletop -- check examples/sigilli [--json]
```

CI runs exactly these. A cold release build takes ~8 min (Typst); prefer `cargo check`
while iterating.

## Typst

- Pinned with `=` (currently 0.15.1) across `typst`, `typst-pdf`, `typst-render`,
  `typst-layout`. Upgrade only in a dedicated branch.
- The API changes between minor versions. Verify signatures in
  `~/.cargo/registry/src/*/typst-*-<version>/` before writing code against them,
  rather than relying on memory or docs for other versions.
- Determinism is a requirement: no system fonts (only Typst's embedded ones and the
  project's fonts), `World::today()` returns `None`, no network. Two builds of the same
  input must produce byte-identical PDFs.

## Conventions

- Code is self-documenting: no explanatory or tutorial comments. Doc comments only
  where a public item's contract is not obvious from its signature.
- Edition 2024, two crates. Add a crate only when a boundary has proven real.
- Diagnostics are the product:
    - every diagnostic has a stable `code` (`data.enum`, `manifest.schema`, …); never
      rename an existing code;
    - errors about data point at the data file (line, and column for YAML) and carry
      `component / instance / field`, never at generated Typst;
    - add hints when the fix is mechanical (allowed values, `did you mean`, quoting).
- Schemas are a closed subset of JSON Schema: new keywords must be added to the lint
  whitelist explicitly. `id` and `qty` are reserved; records are strict by default.
- User-facing CLI output and diagnostics are in English.

## Tests

- Unit tests next to the code; integration tests in `crates/rustabletop-core/tests/` with
  on-disk fixtures in `tests/fixtures/<case>/` (minimal manifest + data + stub face).
- Every milestone exit criterion has a named test (e.g. `m1_exit_criterion_…`).
- Assert on diagnostic `code`, location line and subject, not on message wording.

## Git

- One branch per milestone (`m<n>-<topic>`), PR into `main`, CI green before review.
- Commit messages: imperative summary, then a bullet list of what changed.