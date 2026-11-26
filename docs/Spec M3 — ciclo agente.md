# rustabletop — Spec M3: ciclo agente

**Stato:** approvata (25/11/2026), implementazione da avviare. Base: `main` dopo M2.6 (PR #5).
**Contesto:** "Decisioni v0.1.md" §3.6 e §5. Il formato di manifest, dati e template è chiuso per card, token e board (Spec M2, M2.5, M2.6): M3 non lo cambia, a parte le aggiunte di §6 e §7.

---

## 0. Obiettivo

Un agente che non ha mai visto rustabletop, con a disposizione solo la skill e il binario `rustabletop`, crea un mini-gioco stampabile con carte, gettoni e plancia **senza interventi umani** (criterio misurabile in §9).

Tutto quello che segue serve quel criterio. In ordine di peso: `rustabletop new` (§3), distribuzione del binario (§8), protocollo di valutazione (§9).

## 1. `rustabletop schema [PROJECT] --json`

La descrizione completa del progetto, cioè la prima chiamata che un agente fa su un progetto esistente:

```json
{
  "game": "sigilli",
  "print": { "paper": "a4", "margin": "5mm", "duplex-edge": "long" },
  "icons": ["fuoco", "acqua", "terra", "aria"],
  "components": [
    {
      "id": "vortex", "kind": "token",
      "shape": { "type": "hex", "orientation": "pointy", "across-flats": "30mm" },
      "bleed": "2mm",
      "faces": { "front": "templates/vortex.typ" },
      "print": { "sides": "single", "cut-lines": true, "gap": "2mm" },
      "grid": null,
      "data": null, "instances": 1,
      "schema": { "...": "schema effettivo, con id e qty" }
    }
  ],
  "formats": ["rich-text", "asset", "grid-cell"]
}
```

- Le lunghezze sono stringhe con unità, come nel manifest. Lo schema è quello effettivo, con i campi riservati e `additionalProperties`.
- Funziona anche su un progetto con errori: restituisce quello che è riuscito a risolvere più le diagnostiche, con lo stesso formato del report di `check`.
- Senza `--json` stampa un riassunto leggibile.

## 2. `rustabletop render <component>/<instance> [PROJECT]`

```
rustabletop render dungeon-map/dungeon-map --face front --out preview/ --ppi 150 --json
```

- Compila **solo** l'istanza richiesta e scrive un PNG per faccia (tutte, se manca `--face`). Report `{ ok, artifacts, diagnostics }`, overflow compresi.
- Per le board produce l'intera plancia in una sola immagine, non le sezioni.
- `--ppi` di default 150: basta per un controllo visivo e mantiene leggere le immagini che l'agente deve guardare.
- Il vecchio `rustabletop render <entry.typ>` di basso livello (usato solo dallo spike) diventa `rustabletop typst <entry.typ>`, nascosto da `--help`. Va aggiornato il README dello spike.

## 3. `rustabletop new <dir>`

Crea un progetto iniziale **funzionante**, su cui l'agente lavora modificando invece di partire da zero.

```
rustabletop new my-game [--paper a4|letter]
```

- Contenuto: `game.yaml` con tre componenti, cioè una carta (`card`, poker, fronte e retro), un gettone (`token`, `circle`, piega e incolla) e una plancia (`board`, griglia `hex` con `cell` preso dal gettone, `fields` inline). In più i dati YAML con 2-3 istanze, un template per componente, due icone SVG e un `.gitignore` con `out/`.
- Font: solo quelli incorporati in Typst, niente cartella `fonts/`. L'avvio resta leggero e senza problemi di licenza.
- I template usano **tutto quello che la skill insegna**: `ctx.trim`/`ctx.shape` invece di misure scritte a mano, `safe-area`, `fit(…, field:)`, `rich`, `art` con segnaposto, `ctx.draft`, `draw-grid` e `ctx.components`. È l'esempio canonico: se la skill e il progetto iniziale divergono, vale il progetto iniziale.
- I file sono **incorporati nel binario**. `rustabletop new` su una cartella non vuota fallisce, senza sovrascrivere niente.
- Test: `rustabletop new` seguito da `check` e `build` non produce né errori né avvisi.

## 4. Skill

Si trova in `skills/rustabletop/` e ha una struttura a strati, così da non consumare contesto inutilmente:

```
skills/rustabletop/
├── SKILL.md                 ciclo di lavoro, regole d'oro, quando leggere i riferimenti
└── reference/
    ├── manifest.md          tutte le chiavi del manifest, per kind
    ├── templates.md         contratto della faccia, ctx, stdlib, pattern per card/token/board
    ├── diagnostics.md       codice → significato → correzione tipica
    └── printing.md          sides, fold, tiles, bozza: cosa produce e come si monta
```

**`SKILL.md`** (corto, idealmente sotto le 150 righe):
- **Installazione** (§8) e verifica con `rustabletop --version`.
- **Ciclo di lavoro:** `rustabletop new` → modifica dei dati e dei template → `rustabletop check --json` → `rustabletop render <id>` su ciò che hai toccato → **guarda il PNG** → `rustabletop build`.
- **Regole d'oro:**
  - dati in YAML;
  - non impostare mai la pagina nei template;
  - dimensioni da `ctx`, non numeri scritti a mano;
  - `fit` su ogni testo di lunghezza variabile;
  - un `render.overflow` si corregge accorciando il testo, non abbassando `min`;
  - dopo ogni modifica a un template, guardare il render;
  - rispettare `ctx.draft`.
- **Coordinate delle griglie:** offset `(col, row)`, odd-r per `pointy` e odd-q per `flat`.

**Coerenza verificata da test:**
- ogni codice diagnostico emesso dal codice compare in `reference/diagnostics.md`;
- ogni chiave del manifest accettata dal parser compare in `reference/manifest.md`.

## 5. `render.low-resolution`

- `art()` comunica la dimensione a cui l'immagine viene stampata (tramite metadato, con lo stesso meccanismo di `fit`). Rust legge le dimensioni in pixel dal file (PNG/JPEG; gli SVG sono esclusi) e calcola i dpi effettivi.
- Sotto i **200 dpi** emette un avviso localizzato sul record e sul campo dell'asset:

```
warning[render.low-resolution]: `art` of dungeon-map/dungeon-map prints at 96 dpi (needs ≥ 200)
  --> game.yaml:24
  hint: at this size the image should be at least 3937 × 2756 px
```

## 6. Modalità bozza

- `rustabletop build --draft` e `rustabletop render --draft`: `ctx.draft` vale `true` in tutti i template.
- `art()` in bozza disegna solo il contorno del segnaposto, senza riempimento e senza immagine. Deve saperlo senza ricevere `ctx`: per esempio da un file virtuale del pacchetto `@rtt/std` servito dal `World`, da decidere in implementazione.
- Convenzione per i template, insegnata dalla skill e usata dal progetto iniziale: in bozza niente sfondi pieni, solo contorni e testo.
- Il PDF in bozza si chiama `<game>-pnp-draft.pdf`, per non confonderlo con quello finale.

## 7. Aggiunte al formato

M3 aggiunge solo `ctx.draft` (§6). Tutto il resto del formato resta invariato.

## 8. Distribuzione del binario

- Workflow di **release** su tag `v*`: compila `rustabletop` per Linux x86_64 e macOS aarch64 (Windows facoltativo) e pubblica gli archivi nella release di GitHub.
- `install.sh` nella root del repo: scarica con `curl` il binario della piattaforma corrente dall'ultima release pubblica e lo installa in `~/.local/bin`. Il repo è pubblico dal 25/11/2026, quindi non serve autenticazione. Uso previsto nella skill: `curl -fsSL https://raw.githubusercontent.com/lm98/rustabletop/main/install.sh | sh`.
- Lo script deve fallire con un messaggio chiaro se la piattaforma non è supportata o se non esiste ancora una release, indicando il ripiego `cargo install`.
- La skill riporta l'installazione in una riga, più il ripiego `cargo install --git … rustabletop`, che richiede circa 8 minuti.
- Prima release: `v0.1.0-alpha.1`, alla chiusura di M3.

## 9. Criterio di uscita: valutazione con tre brief

**Protocollo.** Ogni brief viene eseguito in una **sessione nuova**, con solo la skill e `rustabletop` installato tramite `install.sh`. Il messaggio iniziale è il brief e nient'altro.
- Conta come **intervento** qualsiasi messaggio umano successivo al brief.
- L'agente prende da solo le decisioni di design. Se si ferma a fare domande di design, conta come intervento.

**I tre brief:**
1. **Push-your-luck:** 30 carte in 3 semi con valori 1-10, 12 gettoni moneta fronte-retro, nessuna plancia.
2. **Controllo territoriale:** plancia esagonale 10×8 con 6 tipi di terreno, 2 set da 8 gettoni esagonali (uno per giocatore), 12 carte obiettivo.
3. **Dungeon crawl in solitario:** 20 carte stanza con illustrazioni segnaposto, 10 gettoni salute, una plancia con traccia del turno disegnata dal template.

**Successo di ogni brief:**
- zero interventi;
- `rustabletop check` senza errori e `rustabletop build` senza avvisi;
- PDF e PNG coerenti col brief (verifica visiva umana).

**Soglia:** **almeno 2 brief su 3** superati. Ogni fallimento produce una correzione alla skill, al progetto iniziale o alle diagnostiche; il brief fallito si ripete dopo la correzione.

**Tracciabilità:** per ogni esecuzione, un file `docs/evals/m3/<brief>-<n>.md` con il brief, l'esito, gli interventi (se ci sono), le diagnostiche incontrate e le correzioni apportate.

## 10. Criteri tecnici di uscita (oltre a §9)

- Test: `rustabletop new` → `check` → `build` senza diagnostiche; `rustabletop schema --json` sull'esempio e su un progetto con errori; `rustabletop render` di un'istanza per ogni kind; un test di `render.low-resolution` su un'immagine piccola; un test di bozza (`ctx.draft` arriva ai template e `art` non incorpora l'immagine).
- Test di coerenza della skill (§4).
- Workflow di release eseguito almeno una volta su un tag di prova.
- `examples/sigilli` aggiornato per rispettare `ctx.draft`.
- Build deterministico, CI verde, `Decisioni v0.1.md` aggiornato.

## 11. Fuori da M3

Server MCP, watch mode, export TTS (M4), template dichiarativi (rivalutare con i risultati di §9: se i fallimenti si concentrano nei template Typst, la questione si riapre).

## 12. Emendamento (25/11/2026, a implementazione avviata)

Due punti mancavano nella versione originale. Il primo fa parte di M3; il secondo si può chiudere anche dopo il merge di M3, in una PR separata.

### 12.1 Come la skill arriva all'agente (parte di M3)

> **Superato da Spec M3.1** (26/11/2026): `new` non copia più la skill. La skill si carica nell'account (chat e Claude Code) e la produce il binario con `skill export`.

La skill va **incorporata nel binario**, come il progetto iniziale, così la sua versione coincide sempre con quella del tool.

- **`rustabletop new <dir>`** scrive la skill anche in `<dir>/.claude/skills/rustabletop/`. Claude Code carica automaticamente le skill da quella cartella, quindi ogni sessione aperta sul progetto del gioco la trova senza configurazione.
- **`rustabletop skill install [--dir <path>]`**: scrive la skill in `<path>/.claude/skills/rustabletop/`; il default è la directory corrente. Serve per i progetti esistenti. Se trova una versione diversa la sovrascrive e lo segnala nel report.
- La **release** pubblica anche `rustabletop-skill.zip`, che contiene la cartella della skill, per caricarla nelle impostazioni delle skill di claude.ai.
- **Protocollo di valutazione (§9):** la sessione nuova riceve la skill tramite `install.sh` seguito da `rustabletop new` (o `skill install`), non per copia manuale. Il percorso che si valuta è quello che userà un utente vero.
- Test: dopo `rustabletop new`, il contenuto di `.claude/skills/rustabletop/` è identico a quello di `skills/rustabletop/` nel repo.

### 12.2 Una sola documentazione utente (dopo M3)

> **Chiuso in M3.1** (26/11/2026, Spec M3.1 §5): il README è l'ingresso e linka `crates/rustabletop-core/skill/reference/`.

- I file `skills/rustabletop/reference/*.md` diventano **l'unica documentazione utente** (in inglese).
- Il README si riduce a presentazione, installazione, avvio rapido (`install.sh` → `rustabletop new` → `build`) e link a `skills/rustabletop/reference/`. Le sezioni di riferimento oggi presenti nel README (kind, `ctx`, libreria, griglie, stampa) si spostano lì, senza duplicati.
- Ruoli dei documenti: `docs/` è il design (italiano, per chi sviluppa); `skills/rustabletop/` è il riferimento utente (inglese, per agenti e persone); il README è l'ingresso.
