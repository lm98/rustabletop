# rustabletop — Decisioni v0.1

**Stato:** M0–M2.6 completati (M2 PR #2, M2.5 PR #3, M2.6 PR #5; 25/11/2026). M3 in corso sul branch `m3-agent-loop`: implementazione e test tecnici (§10) completati, mancano la release di prova e la valutazione con tre brief (§9), vedi "Spec M3 — ciclo agente.md".
**Sostituisce, per lo scope:** le sezioni 5–6, 9, 11–12 del "Design iniziale". I principi delle sezioni 1–2 restano validi come direzione di lungo periodo.

---

## 1. Premesse (dalle risposte alla review)

| Domanda | Risposta | Conseguenza |
|---|---|---|
| Playtest? | Separato dal framework: stampa fisica o Tabletop Simulator | Il framework produce **artefatti**: niente motore di regole né simulazione. L'export TTS sale di priorità. |
| Per chi è? | Primario: dare a Claude la possibilità di generare i giochi stampabili progettati insieme. Pubblico in futuro, dopo affinamento | **Il primo utente è un agente.** Generalità e polish pubblico vengono dopo. |
| Motore di rendering? | Solo un mezzo | **Typst** come backend di layout, testo ed export. Nessuna Visual IR proprietaria. |
| A cosa serve? *(23/11)* | Prototipazione rapida: i primi 1-3 prototipi di un gioco | Set minimo di componenti: **card, token, board**. Niente dadi come kind. |

## 2. Principio guida

> rustabletop è lo strato **agent-first** tra i dati di un gioco e i suoi artefatti stampabili: valida i dati contro uno schema con diagnostiche che puntano ai dati, delega il disegno a Typst e si occupa di tutto ciò che sta attorno alla singola faccia (istanze, quantità, imposition, export).

Il valore non sta nel "dati → carte → PDF", già coperto da nanDECK, Squib, Component Studio e altri. Sta nel **ciclo agente**: lo schema è introspezionabile, le diagnostiche sono strutturate e localizzate, l'anteprima è renderizzata e consultabile da un modello multimodale.

## 3. Decisioni tecniche

### 3.1 Architettura
- **Rust**, workspace a 2 crate: `rustabletop-core` (lib) e `rustabletop` (binario `rustabletop`). Nuovi crate solo quando un confine si è dimostrato reale.
- **Typst incorporato come libreria** (`typst`, `typst-pdf`, `typst-render`, `typst-layout`), versione **fissata con `=`** (oggi 0.15.1). Gli upgrade sono deliberati: l'API cambia tra versioni minori (verificato nello spike: path e FileId riscritti in 0.15).
- `World` proprietario: file confinati alla root del progetto, **niente font di sistema** (solo quelli incorporati in Typst + `fonts/` del progetto), `today() = None`. Risultato: build deterministico.

### 3.2 Modello dei componenti *(rivisto il 23/11)*
- **Project**: manifest YAML (`game.yaml`).
- **Component** = **forma + insieme di facce + modalità di produzione**. `kind` (`card`, `token`, `board`) è un preset che fornisce i default di tutte e tre, più eventuali vincoli aggiunti allo schema. Dettagli e contratti per kind in "Spec M2.5".
- **Instance**: un record di dati (riga CSV, voce YAML, oppure `fields` inline per i componenti singoli come le plance) con `id` stabile e `qty`.
- **Face**: una funzione Typst `nome(item, ctx)` che disegna su riquadro della forma + 2·bleed. `ctx` porta `kind`, `shape`, `trim`, `bleed`, `component`, `face`, `item`.
- `back` è opzionale per tutti i kind. Per le carte il caso comune è stampare solo i fronti e incollarli o imbustarli.
- Configurazione di stampa divisa in due livelli: **progetto** (stampante: `paper`, `margin`, `duplex-edge`) e **componente** (produzione: `sides`, `crop-marks`, `cut-lines`, `mode`).
- Il "Type vs Instance" del design iniziale resta valido. Le "capabilities" del design iniziale ritornano in forma ridotta, come proprietà dei preset di kind.

### 3.3 Schema e dati
- Lo schema è un **sottoinsieme di JSON Schema** scritto in YAML, validato con il crate `jsonschema`. Gli LLM lo conoscono già, perché è il formato degli schemi dei tool.
- I dati scritti dall'agente vanno in **YAML** (formato primario). Il **CSV** è supportato per l'import da fogli di calcolo.
- Le diagnostiche puntano a **file di dati + riga/chiave + campo**, non al sorgente Typst. Il codice `typst` resta solo per errori nei template.
- *(M1)* `id` e `qty` sono **campi riservati** iniettati dal framework: `id` è uno slug minuscolo obbligatorio, `qty` un intero ≥ 0 con default 1. Lo schema utente non può ridefinirli.
- *(M1)* **Rigore per default**: `additionalProperties: false` salvo diversa indicazione, così un campo scritto male (`cots`) è un errore e non un dato ignorato.
- *(M1)* Il **sottoinsieme di JSON Schema è chiuso**: le parole chiave non elencate vengono rifiutate, con suggerimento. JSON Schema ignorerebbe un `enumm` lasciando il campo senza vincoli.
- *(M1)* Nel CSV i valori sono convertiti **secondo il tipo dichiarato** nello schema; un valore non convertibile resta stringa e produce un errore di tipo localizzato.
- *(M1)* Ogni diagnostica ha un **codice stabile** (`data.enum`, `data.type`, `data.required`, `data.unknown-field`, `data.duplicate-id`, `manifest.schema`, `csv.syntax`, …) e suggerimenti (`allowed values`, `did you mean`).
- *(M2)* Formati custom nello schema: `format: rich-text` (le icone `:nome:` devono essere dichiarate) e `format: asset` (il file deve esistere), verificati da `rustabletop check`.

### 3.4 Template
- **Il linguaggio dei template è Typst**, senza un linguaggio di layout YAML intermedio. Questo abbandona l'idea di "template visuali dichiarativi" del design iniziale (sez. 6.5, 7): Typst è già deterministico, sandboxed e dichiarativo quanto serve.
- Stdlib `@rtt/std` (M2): `rich`, `fit` (segnala l'overflow come diagnostica sul record), `safe-area`, `art`. In M2.5 si aggiunge `outline` e `safe-area` diventa il rettangolo inscritto nella forma.
- Icone come **file SVG/PNG dichiarati nel manifest**.
- I dati arrivano ai template tramite **file virtuali serviti dal `World`**, già validati. Nessun file temporaneo.

### 3.5 Produzione ed export
- Layout pianificato in Rust e disegnato da un modulo Typst generato. In M2.5 diventa una strategia per kind:
  - **card**: griglia accostata, bleed solo sui lati senza vicino, crocini, retro in duplex;
  - **token**: griglia distanziata, linee di taglio sul contorno, retro con **piega e incolla** (default) o duplex;
  - **board**: suddivisione in fogli con sovrapposizione e segni di registro, oppure poster su pagina singola.
- Output v0.1: PDF print-and-play (A4/Letter), PNG per faccia, export TTS.

### 3.6 Integrazione agente
- **Prima la CLI, poi MCP.** Comandi con `--json`: `schema`, `check`, `build`, `render` (anteprima PNG di una singola istanza).
- Una **skill Claude** nel repo (`skills/rustabletop/SKILL.md`) descrive formato, stdlib e ciclo di lavoro per **tutti e tre i kind**. Funziona in Claude Code e in qualsiasi sessione con una shell.
- Il server MCP sarà un adapter sottile sopra `rustabletop-core`, dopo la v0.1.

### 3.7 Pubblicazione *(25/11/2026)*
- Repo pubblico (`lm98/rustabletop`). Licenza **MIT OR Apache-2.0**, come da convenzione dell'ecosistema Rust e compatibile con Typst (Apache-2.0); file `LICENSE-MIT` e `LICENSE-APACHE` nella root.
- I font nei progetti di esempio sono OFL, con il loro file di licenza accanto.
- Stato dichiarato nel README: pre-alpha, formato non stabile fino a `v0.1.0`.
- **Nome:** il binario si chiama `rustabletop`, come repo e crate (prima era `rtt`, che collideva con il Real-Time Transfer dell'ecosistema embedded). Crate: `rustabletop-core` (lib) e `rustabletop` (CLI). Per un tool agent-first la lunghezza non pesa: conta che il nome sia univoco. Il namespace del pacchetto Typst resta `@rtt/std`, perché è virtuale e servito solo dal nostro `World`, quindi non collide con niente.

## 4. Fuori scope v0.1 (con condizione per riconsiderare)

| Elemento | Riconsiderare quando… |
|---|---|
| Grafo di relazioni tipizzate | un export o un validatore ha bisogno di una relazione diversa da "mazzo contiene N copie" |
| Capabilities come sistema aperto | i preset di kind non bastano più (es. tanti kind con proprietà condivise) |
| Plugin / Expander / registry | un componente non è esprimibile come "forma + N facce Typst" |
| WASM, migrazioni, DSL | ci sono utenti esterni con progetti da far evolvere |
| Visual IR / layout engine proprietari | mai, finché Typst regge |
| **Dadi come kind** | mai per la prototipazione: le facce custom sono adesivi su dadi neutri, cioè token quadrati. Eventualmente Custom Dice in TTS (M4+) |
| Forme da SVG, standee, scatole | un prototipo reale ne ha bisogno; il formato `shape: { path: … }` lo permette già |
| Simulazione/playtest automatico | fuori dal progetto |

## 5. Milestone

| # | Contenuto | Criterio di uscita |
|---|---|---|
| **M0** ✅ | Spike Typst puro + spike Typst in Rust | Fatto: vedi §6 |
| **M1** ✅ | Manifest, schema, caricamento YAML/CSV, `rustabletop check --json` | `qty: three` e `suit: aira` producono due diagnostiche che indicano file, riga e campo, con i valori ammessi |
| **M2** ✅ | Contratto delle facce, `@rtt/std`, dati virtuali, imposition generata, `rustabletop build` (PR #2) | Il mazzo dello spike si ricostruisce dal manifest; overflow e icone sconosciute sono diagnostiche localizzate; build deterministico |
| **M2.5** ✅ | Modello dei componenti generalizzato + kind `token` | Fatto: carte con lo stesso layout di M2, Sigillo `circle`/`fold` e Vortice `hex` a faccia singola in `examples/sigilli` con linee di taglio, test del fold, `manifest.face` sulle facce sconosciute, rettangolo inscritto testato; build deterministico |
| **M2.6** ✅ | Kind `board` (tiles / poster, `fields` inline), griglie `hex`/`square` con `format: grid-cell`, geometria condivisa (`ctx.components`, `grid.cell: <id>`), istruzioni di montaggio, linee di taglio a doppia traccia | Fatto: `dungeon-map` 500×350 in `examples/sigilli` con `fields` inline e griglia `hex` da `vortex`, 4 sezioni A4 + pagina di montaggio; test di copertura delle sezioni, `poster`, griglia, `data.grid-cell`, `manifest.grid` con `did you mean`, `back` su `tiles`; doppia traccia verificata sui render; build deterministico |
| **M3** | Ciclo agente: `rustabletop schema`, `rustabletop render <id>`, `rustabletop new`, skill a strati per card/token/board, `render.low-resolution`, modalità bozza (`ctx.draft`), binari precompilati con `install.sh` | Valutazione con 3 brief in sessioni nuove: almeno 2/3 senza interventi, con check e build puliti (Spec M3 §9) |
| **M4** | Export TTS (Deck, Custom Token, Custom Board) | Il gioco si importa in TTS ed è giocabile |

Backlog dopo v0.1: watch mode, MCP, forme da SVG, disposizione a nido d'ape degli esagoni, export SVG di taglio, pacchetti Typst Universe, pipeline di asset/illustrazioni, preset print-on-demand, pubblicazione.

## 6. Risultati degli spike M0

**Setup:** mazzo di prova di 10 carte / 21 istanze, 63×88 mm, bleed 3 mm, A4.

1. **Typst copre tutto il rendering necessario.** Testo con shaping, auto-fit, icone in linea, clipping, mm nativi, PDF/PNG. Lo spike in Typst puro (facce + imposition + pagina per carta) sta in circa 170 righe.
2. **Il build è deterministico.** Con font del progetto e `today = None`, tre build consecutivi danno un PDF identico byte per byte. Tempo: circa 35 ms per il PDF intero (release build).
3. **Gli errori sui dati sono inutilizzabili senza il framework.** `qty: three` produce `string contains invalid digits` con posizione `pnp.typ:5:42`. Il seme errato `aira` non viene nemmeno raggiunto. → giustifica M1.
4. **Il CSV è fragile per un agente.** Il primo errore dello spike è stato un campo con virgole non quotato, scritto da Claude. → YAML come formato primario per i dati generati.
5. **L'imposition è sottile e non spetta all'autore dei template.** Il bleed su A4 con 3×3 carte poker è possibile solo accostando le carte. Nei fogli incompleti servono bleed sui lati senza vicini e retri specchiati. Due bug trovati solo guardando il render. → imposition di proprietà del framework (§3.5).
6. **Le icone in linea alterano l'interlinea** se non sono impaginate con `place` dentro una box di altezza fissa. `fit-text` va scritto con un ciclo `measure`. → entrambi nella stdlib.
7. **L'anteprima visiva è stata indispensabile.** I bug 5 e 6 sono emersi solo guardando i PNG. → `render` è un comando centrale del ciclo agente, non un extra.

## 6b. Note da M1

- La validazione di un progetto è istantanea; il costo di build sta tutto nella compilazione di Typst (circa 8 minuti da zero in release; cache in CI).
- Il manifest si ferma al primo errore (limite di serde), mentre i dati riportano tutti gli errori in una sola passata. Per un agente conta il secondo caso: il manifest cambia raramente.
- Il caso della virgola non quotata nel CSV ora produce `csv.syntax` con l'indicazione di quotare il campo.

## 6d. Note da M2.5

- Il rettangolo inscritto (`ctx.shape.inscribed`) è calcolato e testato in Rust: per il cerchio è il quadrato di lato d/√2; per l'esagono pointy è largo quanto la distanza tra i lati e alto la metà del riquadro (flat: trasposto). È la soluzione di area massima.
- L'imposition dei token centra la griglia nell'area utile. Nel `fold` la griglia occupa la metà sinistra, fino a `gap/2` dall'asse, e i retri sono la riflessione delle celle rispetto all'asse. Le linee di taglio si stampano su entrambe le metà.
- `cut-lines` vale true per default su tutti i token, compresi i `square`: con le celle distanziate serve comunque una guida per il taglio.
- Codice `manifest.size` riusato per gli errori sulla `size` delle forme; `manifest.shape` per `shape`, `orientation` e `radius`.

## 6e. Note da M2.6

- La griglia è interamente calcolata in Rust: `ctx.grid.centers` contiene già i centri in coordinate del canvas, e la stdlib (`draw-grid`, `cell-center`, `cells`, `cell-shape`) si limita a leggerli. I test verificano la dimensione delle celle uguale al token e la distanza `cell` tra i centri di tutte le celle adiacenti, per pointy e flat.
- Le sezioni dei `tiles` hanno una cornice di 7 mm oltre il margine della stampante, per etichette, crocini del rifilo della plancia e segni di registro. L'orientamento della pagina (verticale/orizzontale) è scelto per minimizzare il numero di sezioni: la plancia 500×350 su A4 sta in 2×2 sezioni orizzontali.
- La sovrapposizione è esattamente `overlap` tra sezioni consecutive; l'ultima sezione di ogni riga/colonna è più stretta. `overlap` è limitato a 50 mm.
- Etichette: lettera per la riga, numero per la colonna (A1, A2, B1…). Il testo stampato usa DejaVu Sans Mono (incorporato in Typst), che ha il glifo ✂.
- `format: grid-cell` può stare a qualsiasi profondità dello schema (es. `spaces[].at`): i formati custom ora seguono i percorsi annidati, e il campo nella diagnostica è il percorso puntato (`spaces.7.at`).
- Le opzioni di `print` non ammesse per il kind ora puntano alla chiave stessa (es. `/print/crop-marks`), non a `print`.
- La pagina di montaggio usa la prima istanza; con `qty > 1` le sezioni si ripetono per ogni copia.

## 6f. Note da M3 (implementazione)

- **Gettone del progetto iniziale esagonale, non `circle`.** La spec §3 chiede un gettone `circle` e una griglia `hex` con `cell` preso dal gettone, ma per le regole di M2.6 una griglia `hex` eredita la cella solo da un token `hex`. Il progetto iniziale usa quindi un `hex` pointy da 25 mm con `sides: fold`: insegna il caso più utile (plance esagonali con gettoni della stessa misura) senza cambiare il formato.
- `rustabletop render` lavora sul progetto parziale: rende l'istanza richiesta anche se altri componenti hanno errori (ne importa solo i template) e riporta tutte le diagnostiche del progetto; `ok` è falso se ce n'è almeno una di errore. Accetta `<component>` da solo quando il componente ha una sola istanza. Nuovo codice `render.target` (componente, istanza o faccia sconosciuti, con `did you mean`).
- `rustabletop schema` descrive le forme con le chiavi del loro significato (`width`/`height`/`radius`, `diameter`, `side`/`radius`, `orientation`/`across-flats`) e aggiunge `copies` accanto a `instances`.
- Bozza: `@rtt/std` importa un file virtuale `mode.typ` (`#let draft = …`) servito dal `World`, così `art()` conosce la modalità senza ricevere `ctx`. In bozza `art` non emette metadati, quindi niente `render.low-resolution`.
- `render.low-resolution`: dpi effettivi secondo `fit` (`cover`/`stretch`: asse peggiore; `contain`: asse che riempie il riquadro). Le dimensioni in pixel si leggono con `imagesize`, già presente nel grafo delle dipendenze di Typst. Il campo si ricava confrontando il percorso con i campi `asset` del record.
- Il suggerimento di `render.overflow` ora invita ad accorciare il testo prima di abbassare `min`, come la regola d'oro della skill.
- Skill: incorporata nel binario con `include_str!` da `skills/rustabletop/`, scritta da `new` e da `skill install` (stato `created`/`unchanged`/`updated` nel report). I test di coerenza ricavano le chiavi del manifest dai messaggi `unknown field` di serde e i codici dalle chiamate a `Diagnostic::error/warning`.
- Release: Linux compilato su `ubuntu-22.04` (glibc 2.35, compatibile con le distribuzioni recenti), macOS su `macos-14`, Windows x86_64 (MSVC) su `windows-latest` come zip; `install.sh` copre Windows da Git Bash/MSYS (serve `unzip`), `install.ps1` da PowerShell (`irm …/install.ps1 | iex`; aggiunge `~\.local\bin` al PATH utente, disattivabile con `RUSTABLETOP_NO_MODIFY_PATH`). I tag con `-` sono prerelease; `install.sh` installa l'ultima release, prerelease comprese (oppure `RUSTABLETOP_VERSION`).
- Versione del workspace portata a `0.1.0-alpha.1`, la prima release prevista da §8.

## 7. Rischi

- **Churn dell'API Typst:** versione fissata, upgrade in branch dedicati.
- **Familiarità degli LLM con Typst inferiore a HTML/SVG:** la stdlib riduce la superficie da scrivere, la skill include esempi completi.
- **Clip su forme non rettangolari:** verificato in M2.5, Typst 0.15.1 non ha clip su path arbitrario (solo `radius` sui box). Si usa il clip rettangolare sul riquadro + bleed e la linea di taglio sul contorno.

## 8. Questioni aperte

- **Template dichiarativi in futuro?** Valutare solo se i template Typst si rivelano un collo di bottiglia per l'agente (misurabile in M3).
- **Asset e illustrazioni:** in parte risolta da `format: asset` (M2): file verificato dal check, segnaposto se il campo opzionale manca. Restano da definire le versioni per stampa e TTS.
