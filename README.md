# rustabletop

Agent-first framework for turning tabletop game data into printable and digital artifacts.
You (or an LLM agent) write data in YAML, a JSON Schema subset and [Typst](https://typst.app) face
templates; `rustabletop` validates the data, expands instances, lays out the sheets and exports a
print-and-play PDF plus one PNG per face. Problems come back as structured diagnostics that point at
the data, and every command takes `--json`.

**Status: pre-alpha.** The manifest and template format may still change before `v0.1.0`.

## Install

| | |
|---|---|
| Prebuilt, with cargo | `cargo binstall rustabletop --version 0.1.0-alpha.2` |
| Linux x86_64, macOS arm64, Windows x86_64 (Git Bash) | `curl -fsSL https://raw.githubusercontent.com/lm98/rustabletop/main/install.sh \| sh` |
| Windows PowerShell | `irm https://raw.githubusercontent.com/lm98/rustabletop/main/install.ps1 \| iex` |
| From source (~8 minutes) | `cargo install rustabletop --version 0.1.0-alpha.2` |

The scripts install the latest release into `~/.local/bin`; set `RUSTABLETOP_VERSION` (e.g.
`v0.1.0-alpha.2`) for a specific one. Pre-releases need an explicit `--version` with cargo. On
platforms without a prebuilt binary, `cargo binstall` falls back to building from source.

## Install the skill

The skill teaches Claude the format and the working loop. It is embedded in the binary, so it always
matches a precise version, and it checks that version against the binary before starting.

- **claude.ai:** run `rustabletop skill export` and upload the resulting
  `rustabletop-skill-<version>.zip` in Customize > Skills. Releases ship the same file as
  `rustabletop-skill.zip`. In a chat, Claude installs the binary by itself, at the skill's version.
- **Claude Code:** signed in with the same account, the uploaded skill is already there. Without an
  account, `rustabletop skill install --dir ~` writes it to `~/.claude/skills/rustabletop/`
  (`--dir <project>` pins a copy in one project).

After upgrading the binary, export and upload the skill again.

## Quick start

```sh
rustabletop new my-game                    # a working card + token + board project
rustabletop check my-game                  # validate manifest, schemas and data
rustabletop render card/eclipse my-game    # PNG of one instance, one per face
rustabletop build my-game                  # my-game/out/my-game-pnp.pdf (--draft for an ink-saving proof)
```

`rustabletop schema my-game --json` describes a project: components, shapes, print options and
effective schemas. [`examples/sigilli`](examples/sigilli) is a larger reference project.

## Reference

The skill is the user documentation, for agents and people alike:

- [`SKILL.md`](crates/rustabletop-core/skill/SKILL.md): the working loop and the golden rules
- [`reference/manifest.md`](crates/rustabletop-core/skill/reference/manifest.md): every key of `game.yaml` (kinds, shapes, schemas, grids, `print`)
- [`reference/templates.md`](crates/rustabletop-core/skill/reference/templates.md): face templates, `ctx`, the `@rtt/std` library, patterns per kind
- [`reference/diagnostics.md`](crates/rustabletop-core/skill/reference/diagnostics.md): every diagnostic code and its usual fix
- [`reference/printing.md`](crates/rustabletop-core/skill/reference/printing.md): duplex, fold, crop marks, board sections, draft

Design notes, decisions and milestone specs (in Italian) are in [`docs/`](docs/), starting from
[`Decisioni v0.1.md`](docs/Decisioni%20v0.1.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option. The fonts in `examples/sigilli/fonts` and `spike/m0a/fonts` are Cardo and Fira Sans,
under the SIL Open Font License (see the `OFL-*.txt` files).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this
project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
