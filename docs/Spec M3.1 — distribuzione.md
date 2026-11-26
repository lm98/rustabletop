# rustabletop — Spec M3.1: distribuzione di binario e skill

**Stato:** approvata (26/11/2026), implementazione da avviare. Base: `main` **con la PR #10 unita** (skill in `crates/rustabletop-core/skill/`, metadati crates.io, versione `0.1.0-alpha.2`).
**Rilascio:** M3.1 e PR #10 escono insieme come **`0.1.0-alpha.2`**: tag, binari, zip della skill e pubblicazione su crates.io avvengono dopo il merge di M3.1, non prima.
**Contesto:** sostituisce Spec M3 §12.1 per la parte "`new` copia la skill nel progetto" e chiude §12.2.

---

## 0. Il cambio di modello

Spec M3 §12.1 metteva una copia della skill in ogni progetto (`rustabletop new`). Così però la skill non arriva nella chat di claude.ai, e in Claude Code si crea un doppione di versione rispetto alla skill sincronizzata dall'account. Il nuovo modello:

- **Binario:** l'utente lo installa una volta (`cargo binstall rustabletop`, `install.sh`, `cargo install`). Nella sandbox della chat lo installa l'agente, alla versione indicata dalla skill.
- **Skill:** l'utente la carica una volta nelle skill del suo account claude.ai (Customize > Skills). Da lì è disponibile nelle chat e in Claude Code, se accede con lo stesso account. Per Claude Code senza account c'è `skill install --dir ~`.
- **Fonte di verità:** il binario. La skill è incorporata nel binario, e lo zip da caricare lo produce il binario stesso (`skill export`), quindi la skill corrisponde sempre a una versione precisa.
- **Allineamento:** la skill dichiara la propria versione e la confronta con quella del binario al primo passo. Se divergono, l'agente lo dice e prosegue: `schema --json` e `check` vengono dal binario, quindi restano autorevoli anche con una skill vecchia.

## 1. `cargo binstall`

In `crates/rustabletop/Cargo.toml`:

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/v{ version }/{ name }-{ target }{ archive-suffix }"
bin-dir = "{ bin }{ binary-ext }"
pkg-fmt = "tgz"

[package.metadata.binstall.overrides.x86_64-pc-windows-msvc]
pkg-fmt = "zip"
```

- Corrisponde ai nomi degli archivi prodotti dal workflow di release, con il binario alla radice dell'archivio.
- Un test legge `Cargo.toml` e verifica che `pkg-url` corrisponda al nome degli archivi usato in `release.yml`, così una modifica a uno dei due non rompe binstall in silenzio.
- Per le piattaforme senza binario precompilato (per esempio macOS Intel), binstall ripiega sulla compilazione: va bene così.

## 2. `rustabletop skill export`

```
rustabletop skill export [--zip <file>] [--dir <path>] [--json]
```

- `--zip` (default `rustabletop-skill-<version>.zip` nella directory corrente) scrive uno zip con la cartella `rustabletop/` alla radice, il formato richiesto dall'upload di claude.ai (la cartella ha il nome della skill).
- `--dir <path>` scrive la cartella `rustabletop/` in `<path>`, utile per ispezionarla.
- Il workflow di release produce `rustabletop-skill.zip` con `rustabletop skill export` (il binario Linux appena compilato), non più copiando i file: una sola fonte.
- Test: lo zip esportato contiene `rustabletop/SKILL.md` e tutti i file di `reference/`, e la versione nel testo è quella del binario (§3).

## 3. Versione nella skill e controllo al primo passo

- `SKILL.md` nel repo contiene il segnaposto `{{RUSTABLETOP_VERSION}}`, che il binario sostituisce quando scrive la skill (`skill export`, `skill install`). Il repo non va quindi aggiornato a ogni release.
- Test: il file nel repo contiene il segnaposto; nessun file scritto da `export`/`install` lo contiene ancora.
- La sezione **Install** di `SKILL.md` diventa un controllo di versione:

```sh
rustabletop --version   # this skill is for rustabletop {{RUSTABLETOP_VERSION}}
```

  - **Comando non trovato** (tipico della sandbox della chat): installare *esattamente* la versione della skill con `RUSTABLETOP_VERSION=v{{RUSTABLETOP_VERSION}}` passato a `install.sh`, poi aggiungere `~/.local/bin` al `PATH`. Ripiego: `cargo binstall rustabletop --version {{RUSTABLETOP_VERSION}}`, se `cargo` è disponibile.
  - **Versione diversa:** dirlo all'utente **una volta**, con i due rimedi: aggiornare la skill con `rustabletop skill export --zip` e ricaricarla, oppure installare il binario della versione della skill. Poi proseguire, trattando `schema --json` e le diagnostiche come autorevoli.
  - **Versione uguale:** nessun messaggio.
- `install.sh` (già supporta `RUSTABLETOP_VERSION`) stampa alla fine la riga `export PATH=…` quando serve. Va verificato che il percorso con versione esplicita funzioni anche per le pre-release.

## 4. `rustabletop new` senza skill

- `new` non scrive più `.claude/skills/rustabletop/`.
- `rustabletop skill install [--dir <path>]` resta, per chi vuole una copia fissata in un progetto (lavoro offline, collaboratori senza la skill) o a livello utente (`--dir ~` → `~/.claude/skills/rustabletop/`). Il comportamento resta quello di M3 (crea / invariata / aggiornata).
- Test: dopo `new` non esiste `.claude/`; `skill install` scrive lo stesso albero di `skill export --dir`.

## 5. README come ingresso (Spec M3 §12.2)

- Il README si riduce a: cos'è, stato pre-alpha, installazione (binstall, `install.sh`, `install.ps1`, `cargo install`), **installazione della skill** (claude.ai: `skill export --zip` → Customize > Skills; Claude Code: account sincronizzato oppure `skill install --dir ~`), avvio rapido (`new` → `check` → `render` → `build`), link al riferimento e licenza.
- Le sezioni di riferimento oggi presenti nel README (kind, `ctx`, libreria, griglie, stampa) si spostano in `crates/rustabletop-core/skill/reference/`, unendole a quanto c'è già, senza duplicati. Il README linka quei file.
- Ruoli dei documenti: `docs/` è il design (italiano); la skill è il riferimento utente (inglese); il README è l'ingresso.

## 6. Ripiego PyPI (condizionale)

**Si attiva solo se il test di `install.sh` nella chat di claude.ai fallisce** perché la sandbox non raggiunge gli asset delle release di GitHub.

- Wheel con il binario, costruito con `maturin` in modalità `bin`, pubblicato su PyPI dal workflow di release con lo stesso numero di versione (in formato PEP 440: `0.1.0a2`).
- Nella skill, il ramo "comando non trovato" prova prima `pip install rustabletop==<versione PEP 440>`, poi `install.sh`.
- Se il test passa, questa sezione resta nel backlog.

## 7. Criteri di uscita

- `cargo binstall rustabletop --version 0.1.0-alpha.2` installa il binario precompilato senza compilare (verifica manuale dopo la pubblicazione, annotata in Decisioni).
- `skill export --zip` produce uno zip che claude.ai accetta in Customize > Skills (verifica manuale).
- **Chat di claude.ai (verifica manuale):** in una chat nuova, con solo la skill caricata, il brief "Create a printable 10-card deck for a simple trick-taking game" porta Claude a installare il binario alla versione della skill, costruire il progetto e consegnare il PDF, senza interventi.
- **Claude Code in sessione davvero nuova (verifica manuale, chiude la lacuna della valutazione di M3):** con la skill sincronizzata dall'account o installata con `--dir ~`, uno dei tre brief di M3 superato senza interventi. L'esito va in `docs/evals/m3.1/`.
- Test automatici di §1–§4; `new` → `check` → `build` ancora pulito; CI verde.
- Decisioni aggiornato (§3.6, §3.7, riga M3.1) e Spec M3 §12.1 annotata come superata.

## 8. Fuori da M3.1

Anteprima dei fogli (`build --sheets`, proposta dalla valutazione M3), export TTS (M4), server MCP, marketplace di plugin per Claude Code.
