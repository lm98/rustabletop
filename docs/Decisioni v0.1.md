# rustabletop — Decisioni v0.1

**Stato:** M0, M1, M2, M2.5 completati (M2 unito con la PR #2; M2.5 sul branch `m2.5-components`, 23/11/2026). Prossima: M2.6 (`board`), vedi "Spec M2.5 — modello dei componenti e token.md" §4 e §7.
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
- **Rust**, workspace a 2 crate: `rtt-core` (lib) e `rtt-cli` (binario `rtt`). Nuovi crate solo quando un confine si è dimostrato reale.
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
- *(M2)* Formati custom nello schema: `format: rich-text` (le icone `:nome:` devono essere dichiarate) e `format: asset` (il file deve esistere), verificati da `rtt check`.

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
- Il server MCP sarà un adapter sottile sopra `rtt-core`, dopo la v0.1.

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
| **M1** ✅ | Manifest, schema, caricamento YAML/CSV, `rtt check --json` | `qty: three` e `suit: aira` producono due diagnostiche che indicano file, riga e campo, con i valori ammessi |
| **M2** ✅ | Contratto delle facce, `@rtt/std`, dati virtuali, imposition generata, `rtt build` (PR #2) | Il mazzo dello spike si ricostruisce dal manifest; overflow e icone sconosciute sono diagnostiche localizzate; build deterministico |
| **M2.5** ✅ | Modello dei componenti generalizzato + kind `token` | Fatto: carte con lo stesso layout di M2, Sigillo `circle`/`fold` e Vortice `hex` a faccia singola in `examples/sigilli` con linee di taglio, test del fold, `manifest.face` sulle facce sconosciute, rettangolo inscritto testato; build deterministico |
| **M2.6** | Kind `board` (tiles / poster, `fields` inline) | Vedi "Spec M2.5" §7 |
| **M3** | Ciclo agente: `schema`/`render --json`, skill per card, token, board | Claude crea un mini-gioco con carte, token e plancia senza correzioni manuali |
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
- `board` non è ancora un valore ammesso di `kind`: arriva in M2.6.
- Codice `manifest.size` riusato per gli errori sulla `size` delle forme; `manifest.shape` per `shape`, `orientation` e `radius`.

## 7. Rischi

- **Churn dell'API Typst:** versione fissata, upgrade in branch dedicati.
- **Familiarità degli LLM con Typst inferiore a HTML/SVG:** la stdlib riduce la superficie da scrivere, la skill include esempi completi.
- **Clip su forme non rettangolari:** verificato in M2.5, Typst 0.15.1 non ha clip su path arbitrario (solo `radius` sui box). Si usa il clip rettangolare sul riquadro + bleed e la linea di taglio sul contorno.
- **Nome del binario `rtt`:** collide con RTT (Real-Time Transfer, ecosistema embedded/probe-rs). Da rivedere prima della pubblicazione.

## 8. Questioni aperte

- **Template dichiarativi in futuro?** Valutare solo se i template Typst si rivelano un collo di bottiglia per l'agente (misurabile in M3).
- **Asset e illustrazioni:** in parte risolta da `format: asset` (M2): file verificato dal check, segnaposto se il campo opzionale manca. Restano da definire le versioni per stampa e TTS.
- **Licenza:** il workspace dichiara MIT OR Apache-2.0 (Typst è Apache-2.0). Da confermare prima della pubblicazione.
