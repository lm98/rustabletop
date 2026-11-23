# rustabletop — Spec M2.5: modello dei componenti e token

**Stato:** approvata (23/11/2026), implementazione da avviare. Base: `main` dopo il merge di M2 (PR #2).
**Contesto:** "Decisioni v0.1.md" §3.2 e §5. Il contratto della faccia di M2 ("Spec M2 — template e build.md" §1) resta valido; qui si generalizza ciò che attorno alla faccia dà per scontata la carta.

---

## 0. Perché prima di M3

M3 scrive la skill e gli esempi che insegnano il formato agli agenti. Il formato va quindi chiuso **per tutti i kind della v0.1** prima di M3: cambiarlo dopo significa riscrivere skill, esempi e i progetti già generati.

- **Contratti** di card, token e board: definiti qui.
- **Implementazione:** M2.5 = refactoring del modello + `token`. M2.6 = `board`. Poi M3.

## 1. Set di kind della v0.1

Il tool serve ai primi 1-3 prototipi di un gioco. Il set minimo è **card, token, board**.

**Dadi: fuori dal progetto come kind.** Nei playtest di solito si usano dadi che si hanno già in casa. Le facce personalizzate si ottengono con adesivi su dadi neutri: un foglio di token quadrati a faccia singola, già coperto da `token`.

## 2. Modello generale

Un componente è **forma + insieme di facce + modalità di produzione**. `kind` è un preset che fornisce i default di tutte e tre, più eventuali vincoli aggiunti allo schema.

| kind | Forme | Facce | Produzione default | TTS (M4) |
|---|---|---|---|---|
| `card` | `rect` (raggio opzionale) | `front`, [`back`] | griglia accostata + crocini; retro in duplex se presente | Deck |
| `token` | `circle`, `square`, `hex` | `front`, [`back`] | griglia distanziata + linee di taglio; retro con **piega e incolla** | Custom Token |
| `board` | `rect` | `front`, [`back`] | suddivisione in fogli con sovrapposizione | Custom Board / Tile |

`back` è **opzionale per tutti i kind**. Per le carte è già così in M2, ed è il caso d'uso comune: stampare solo i fronti su A4 e incollarli su altre carte o infilarli nelle bustine.

### 2.1 Contratto della faccia (invariato) e `ctx` (esteso)

```typst
#let front(item, ctx) = { ... }
```

La funzione disegna su un'area pari al **riquadro della forma + 2·bleed**, con origine nell'angolo del bleed. `ctx` diventa:

```
(
  kind: "token",
  shape: (type: "hex", orientation: "pointy", width, height, radius: none),
  trim: (width, height),   // riquadro della forma (bounding box)
  bleed,
  component, face, item,   // item = id dell'istanza
)
```

- I template generici usano `ctx.trim` e `ctx.shape` invece di dimensioni scritte a mano.
- Stdlib: `outline(ctx, inset: 0mm, ..args)` disegna il contorno della forma con un margine (cornici che seguono un esagono o un cerchio). `safe-area(ctx, inset:)` diventa il **rettangolo inscritto** nella forma, ridotto del margine.

### 2.2 Bleed e ritaglio su forme non rettangolari

- Il bleed è un **offset del contorno**. Il template deve far debordare lo sfondo oltre il contorno, come per le carte.
- Il framework ritaglia al riquadro + bleed. **Da verificare in Typst 0.15.1** se esiste un clip su path arbitrario. Se non c'è, basta il clip rettangolare: l'inchiostro negli angoli del riquadro cade fuori dalla linea di taglio e le celle distanziate evitano sovrapposizioni.
- Per le forme non rettangolari i crocini non servono: si stampa la **linea di taglio** sul contorno, sottile e opzionale.

### 2.3 Facce: da struct a mappa

```yaml
faces:
  front: templates/token.typ
  back: templates/token.typ
```

- `faces` diventa una mappa `nome → file`, validata sull'insieme di facce del kind. Una faccia sconosciuta produce `manifest.face` con i nomi ammessi e `did you mean`.
- La funzione ha lo stesso nome della faccia (come in M2).
- Il jolly `"*"` (una funzione `face` per tutte le facce) **non serve senza dadi**: non va implementato, ma il formato lo lascia possibile.

### 2.4 Configurazione di stampa: stampante vs componente

M2 ha tutto in `print` a livello di progetto. Va separato ciò che dipende dalla **stampante** da ciò che dipende dal **componente**:

```yaml
print:                  # progetto: proprietà della stampante e della carta
  paper: a4
  margin: 5mm
  duplex-edge: long     # long | short — come gira il foglio questa stampante

components:
  - id: wound-token
    kind: token
    print:              # componente: come si produce questo pezzo
      sides: fold       # duplex | fold | single — default dal kind
      cut-lines: true   # default true per forme non rettangolari
```

- `sides: single` stampa solo i fronti. È implicito se `back` manca.
- Le carte mantengono `crop-marks` (default true) a livello di componente.
- È un cambiamento incompatibile del manifest di M2: va bene ora, prima di M3; serve solo migrare `examples/sigilli`.

### 2.5 `data` opzionale

```yaml
- id: dungeon-map
  kind: board
  fields: { title: "Le Cripte" }   # istanza singola inline, validata dallo schema
```

- `data` assente → **una sola istanza** con `id` uguale all'id del componente, con i campi presi da `fields` (opzionale).
- Anche `schema` diventa opzionale (default: oggetto vuoto, rigoroso).
- `data` e `fields` insieme producono un errore.

## 3. Contratto `token` (implementazione M2.5)

```yaml
- id: wound
  kind: token
  shape: circle          # circle | square | hex
  size: 25mm             # circle: diametro; square: lato; hex: distanza tra i lati
  orientation: pointy    # solo hex: pointy | flat (default pointy)
  bleed: 2mm
  schema: { ... }
  faces:
    front: templates/token.typ
    back: templates/token.typ
  data: data/tokens.yaml
  print: { sides: fold }
```

- `square` accetta anche `radius` per gli angoli arrotondati.
- Le forme da SVG (scudi, standee) **non** sono in M2.5, ma il formato le lascia possibili: `shape: { path: shapes/shield.svg }`.

**Produzione:**
- Griglia di celle = riquadro + 2·bleed + `gap` (default 2mm). I gettoni non condividono le linee di taglio, a differenza delle carte.
- **`sides: fold` (default se c'è `back`).** La pagina è divisa a metà sull'asse verticale: i fronti stanno a sinistra, i retri **specchiati** a destra, e al centro c'è una linea di piega tratteggiata. Si piega il foglio, si incolla e si ritagliano i gettoni attraverso i due strati. È più robusto del duplex sulle stampanti di casa, dove l'errore di allineamento di 1-2 mm su un gettone da 25 mm si vede.
- `sides: duplex`: come per le carte, con il retro specchiato secondo `duplex-edge`.
- La geometria del fold va calcolata e testata in Rust, come l'imposition di M2. Proprietà da testare: piegando lungo l'asse, ogni retro coincide col proprio fronte.

## 4. Contratto `board` (implementazione M2.6)

```yaml
- id: dungeon-map
  kind: board
  size: { width: 500mm, height: 350mm }
  bleed: 3mm
  faces: { front: templates/map.typ }
  fields: { title: "Le Cripte" }
  print:
    mode: tiles          # tiles | poster
    overlap: 10mm
```

- **`tiles`** (default): la faccia è disegnata una sola volta a dimensione piena e ritagliata in sezioni grandi quanto l'area stampabile della pagina, con `overlap` di sovrapposizione. Ogni pagina ha segni di registro agli angoli e un'etichetta della sezione (A1, A2, …) nel margine.
- **`poster`**: una pagina della dimensione della plancia + bleed, per la tipografia o la stampa grande formato. Ignora `paper`.
- `back` sulle board è ammesso solo in modalità `poster` (v0.1). In modalità `tiles` produce un errore con il suggerimento di usare `poster`.
- `qty` resta ammesso (default 1).

## 5. Refactoring del codice di M2

| Oggi (M2) | Diventa |
|---|---|
| `Kind::Card` | `Kind::{Card, Token, Board}` con un preset: facce ammesse, forme ammesse, default di produzione |
| `Faces { front, back }` | `BTreeMap<String, String>` validata sul preset |
| `Size` del componente | `Shape { Rect{w,h,radius}, Circle{d}, Square{side,radius}, Hex{across_flats, orientation} }` con `bbox()` |
| `impose::Side::{Front, Back}` | nome della faccia (`String`) |
| `Cell.card` | `Cell.item` |
| `Grid` + `sheets()` | strategie di layout: `CardGrid` (accostata, crocini), `TokenGrid` (distanziata, linee di taglio, fold/duplex), `Tiles` / `Poster` (board, M2.6) |
| `print` di progetto con `crop-marks`/`duplex` | `print` di progetto (`paper`, `margin`, `duplex-edge`) + `print` di componente |
| `ctx` con `trim`, `bleed`, `component`, `face` | più `kind`, `shape`, `item` |
| `data` obbligatorio | `data` opzionale, `fields` inline |

I codici diagnostici esistenti non cambiano. Nuovi: `manifest.face` (faccia non ammessa per il kind), `manifest.shape`, `manifest.print` (opzione non valida per il kind), `manifest.data` (`data` e `fields` insieme).

## 6. Criterio di uscita M2.5

- `examples/sigilli` migrato al nuovo manifest; le carte producono lo stesso layout di M2 (stesse posizioni delle celle, verificate dal test di imposition).
- `examples/sigilli` aggiunge **due componenti token**: un Sigillo rotondo fronte-retro (`circle`, `fold`) e un gettone esagonale a faccia singola. `rtt build` li impagina con le linee di taglio.
- Test del fold: per ogni fronte, il retro corrispondente è nella posizione speculare rispetto all'asse di piega.
- `faces: { left: ... }` su un token produce `manifest.face` con le facce ammesse.
- `ctx.shape` è disponibile nei template; `outline` e `safe-area` funzionano su `circle` e `hex` (verifica visiva sui PNG + test della geometria del rettangolo inscritto).
- Build deterministico, CI verde.

## 7. Criterio di uscita M2.6

- `examples/sigilli` aggiunge una plancia 500×350 mm senza file di dati (`fields` inline).
- `tiles` su A4 produce sezioni con sovrapposizione, segni di registro ed etichette; ricomponendo le sezioni (test geometrico sulle coordinate) si copre l'intera plancia senza buchi.
- `poster` produce una pagina singola della dimensione della plancia + bleed.

## 8. Fuori da M2.5/M2.6

Forme da SVG, disposizione a nido d'ape degli esagoni, export SVG di taglio per plotter, dadi, standee e scatole.
