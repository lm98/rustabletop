# rustabletop — Spec M3: ciclo agente

**Stato:** approvata (25/11/2026), implementazione da avviare. Base: `main` dopo M2.6 (PR #5).
**Contesto:** "Decisioni v0.1.md" §3.6 e §5. Il formato di manifest, dati e template è chiuso per card, token e board (Spec M2, M2.5, M2.6): M3 non lo cambia, a parte le aggiunte di §6 e §7.

---

## 0. Obiettivo

Un agente che non ha mai visto rustabletop, con a disposizione solo la skill e il binario `rtt`, crea un mini-gioco stampabile con carte, gettoni e plancia **senza interventi umani** (criterio misurabile in §9).

Tutto quello che segue serve quel criterio. In ordine di peso: `rtt new` (§3), distribuzione del binario (§8), protocollo di valutazione (§9).

## 1. `rtt schema [PROJECT] --json`

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

## 2. `rtt render <component>/<instance> [PROJECT]`

```
rtt render dungeon-map/dungeon-map --face front --out preview/ --ppi 150 --json
```

- Compila **solo** l'istanza richiesta e scrive un PNG per faccia (tutte, se manca `--face`). Report `{ ok, artifacts, diagnostics }`, overflow compresi.
- Per le board produce l'intera plancia in una sola immagine, non le sezioni.
- `--ppi` di default 150: basta per un controllo visivo e mantiene leggere le immagini che l'agente deve guardare.
- Il vecchio `rtt render <entry.typ>` di basso livello (usato solo dallo spike) diventa `rtt typst <entry.typ>`, nascosto da `--help`. Va aggiornato il README dello spike.

## 3. `rtt new <dir>`

Crea un progetto iniziale **funzionante**, su cui l'agente lavora modificando invece di partire da zero.

```
rtt new my-game [--paper a4|letter]
```

- Contenuto: `game.yaml` con tre componenti, cioè una carta (`card`, poker, fronte e retro), un gettone (`token`, `circle`, piega e incolla) e una plancia (`board`, griglia `hex` con `cell` preso dal gettone, `fields` inline). In più i dati YAML con 2-3 istanze, un template per componente, due icone SVG e un `.gitignore` con `out/`.
- Font: solo quelli incorporati in Typst, niente cartella `fonts/`. L'avvio resta leggero e senza problemi di licenza.
- I template usano **tutto quello che la skill insegna**: `ctx.trim`/`ctx.shape` invece di misure scritte a mano, `safe-area`, `fit(…, field:)`, `rich`, `art` con segnaposto, `ctx.draft`, `draw-grid` e `ctx.components`. È l'esempio canonico: se la skill e il progetto iniziale divergono, vale il progetto iniziale.
- I file sono **incorporati nel binario**. `rtt new` su una cartella non vuota fallisce, senza sovrascrivere niente.
- Test: `rtt new` seguito da `check` e `build` non produce né errori né avvisi.

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
- **Installazione** (§8) e verifica con `rtt --version`.
- **Ciclo di lavoro:** `rtt new` → modifica dei dati e dei template → `rtt check --json` → `rtt render <id>` su ciò che hai toccato → **guarda il PNG** → `rtt build`.
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

- `rtt build --draft` e `rtt render --draft`: `ctx.draft` vale `true` in tutti i template.
- `art()` in bozza disegna solo il contorno del segnaposto, senza riempimento e senza immagine. Deve saperlo senza ricevere `ctx`: per esempio da un file virtuale del pacchetto `@rtt/std` servito dal `World`, da decidere in implementazione.
- Convenzione per i template, insegnata dalla skill e usata dal progetto iniziale: in bozza niente sfondi pieni, solo contorni e testo.
- Il PDF in bozza si chiama `<game>-pnp-draft.pdf`, per non confonderlo con quello finale.

## 7. Aggiunte al formato

M3 aggiunge solo `ctx.draft` (§6). Tutto il resto del formato resta invariato.

## 8. Distribuzione del binario

- Workflow di **release** su tag `v*`: compila `rtt` per Linux x86_64 e macOS aarch64 (Windows facoltativo) e pubblica gli archivi nella release di GitHub.
- `install.sh` nella root del repo: scarica il binario della piattaforma corrente in `~/.local/bin`. Usa `gh release download` se `gh` è autenticato, altrimenti `curl` sulla URL pubblica.
- **Vincolo: il repo è privato.** Gli asset delle release di un repo privato non si scaricano senza autenticazione. Finché il repo resta privato, l'installazione funziona solo dove `gh` ha accesso al repo (come le sessioni Claude collegate). Il percorso con `curl` funzionerà quando il repo diventerà pubblico.
- La skill riporta l'installazione in una riga, più il ripiego `cargo install --git … rtt-cli`, che richiede circa 8 minuti.
- Prima release: `v0.1.0-alpha.1`, alla chiusura di M3.

## 9. Criterio di uscita: valutazione con tre brief

**Protocollo.** Ogni brief viene eseguito in una **sessione nuova**, con solo la skill e `rtt` installato tramite `install.sh`. Il messaggio iniziale è il brief e nient'altro.
- Conta come **intervento** qualsiasi messaggio umano successivo al brief.
- L'agente prende da solo le decisioni di design. Se si ferma a fare domande di design, conta come intervento.

**I tre brief:**
1. **Push-your-luck:** 30 carte in 3 semi con valori 1-10, 12 gettoni moneta fronte-retro, nessuna plancia.
2. **Controllo territoriale:** plancia esagonale 10×8 con 6 tipi di terreno, 2 set da 8 gettoni esagonali (uno per giocatore), 12 carte obiettivo.
3. **Dungeon crawl in solitario:** 20 carte stanza con illustrazioni segnaposto, 10 gettoni salute, una plancia con traccia del turno disegnata dal template.

**Successo di ogni brief:**
- zero interventi;
- `rtt check` senza errori e `rtt build` senza avvisi;
- PDF e PNG coerenti col brief (verifica visiva umana).

**Soglia:** **almeno 2 brief su 3** superati. Ogni fallimento produce una correzione alla skill, al progetto iniziale o alle diagnostiche; il brief fallito si ripete dopo la correzione.

**Tracciabilità:** per ogni esecuzione, un file `docs/evals/m3/<brief>-<n>.md` con il brief, l'esito, gli interventi (se ci sono), le diagnostiche incontrate e le correzioni apportate.

## 10. Criteri tecnici di uscita (oltre a §9)

- Test: `rtt new` → `check` → `build` senza diagnostiche; `rtt schema --json` sull'esempio e su un progetto con errori; `rtt render` di un'istanza per ogni kind; un test di `render.low-resolution` su un'immagine piccola; un test di bozza (`ctx.draft` arriva ai template e `art` non incorpora l'immagine).
- Test di coerenza della skill (§4).
- Workflow di release eseguito almeno una volta su un tag di prova.
- `examples/sigilli` aggiornato per rispettare `ctx.draft`.
- Build deterministico, CI verde, `Decisioni v0.1.md` aggiornato.

## 11. Fuori da M3

Server MCP, watch mode, export TTS (M4), template dichiarativi (rivalutare con i risultati di §9: se i fallimenti si concentrano nei template Typst, la questione si riapre).
