# rustabletop — Spec M2.6: board, griglie e montaggio

**Stato:** approvata (25/11/2026), implementazione da avviare. Base: `main` dopo M2.5 (PR #3).
**Contesto:** "Decisioni v0.1.md" §3.2 e §5; contratto `board` di base in "Spec M2.5 — modello dei componenti e token.md" §4 e §7, che questa spec **estende e sostituisce** per M2.6.

---

## 0. Perché queste aggiunte prima di M3

Griglie (§3) e geometria condivisa (§4) cambiano il formato del manifest e della stdlib, quindi vanno chiuse prima che M3 lo insegni agli agenti. Le istruzioni di montaggio (§5) non cambiano il formato, ma senza di esse una plancia divisa in sezioni è scomoda da usare. L'avviso di bassa risoluzione è rinviato a M3 (§8).

## 1. Kind `board`

```yaml
- id: dungeon-map
  kind: board
  size: { width: 500mm, height: 350mm }
  bleed: 3mm
  faces: { front: templates/map.typ }
  schema:
    properties:
      title: { type: string }
      spaces:
        type: array
        items:
          type: object
          required: [at, terrain]
          properties:
            at: { format: grid-cell }
            terrain: { enum: [floor, lava, wall] }
  fields:
    title: Le Cripte
    spaces:
      - { at: [3, 4], terrain: lava }
  grid: { type: hex, cols: 14, rows: 9, cell: vortex, origin: [25mm, 30mm] }
  print:
    mode: tiles          # tiles | poster
    overlap: 10mm
```

- Forme: solo `rect`, con raggio opzionale. Facce: `front`, [`back`].
- Valgono le regole già fissate in Spec M2.5 §4: `tiles` è il default; `poster` produce una pagina della dimensione della plancia + bleed e ignora `paper`; `back` è ammesso solo in `poster` (con `tiles` produce `manifest.print` e suggerisce `poster`); `qty` è ammesso, default 1.
- `data` e `fields` funzionano come in M2.5. Il caso tipico è `fields` inline.

## 2. Correzione dalla review di M2.5: linee di taglio

La linea di taglio dei token è oggi `0.3pt + luma(60)`: sparisce sui fronti scuri. Diventa una **doppia traccia**, bianca più larga sotto e nera sottile sopra, visibile su qualsiasi sfondo. Vale per tutte le linee di taglio sul contorno; i crocini delle carte restano invariati, perché stanno sulla carta bianca.

## 3. Griglie

### 3.1 Dichiarazione nel manifest

Una board può dichiarare **una** griglia (`grid`, opzionale):

| Chiave | Valori |
|---|---|
| `type` | `hex` \| `square` |
| `cols`, `rows` | interi ≥ 1 |
| `cell` | una lunghezza (`30mm`) oppure l'**id di un componente** (§4) |
| `orientation` | solo `hex`: `pointy` \| `flat`. Se `cell` è un token esagonale, viene ereditata da lì; dichiararla diversa produce un errore |
| `origin` | `[x, y]`, posizione del centro della cella `(0, 0)` rispetto all'angolo di rifilo; default: griglia centrata nella plancia |

- `cell` per `hex` indica la distanza tra i lati, come `size` dei token esagonali. Per `square` indica il lato.
- Coordinate **offset `(col, row)`**, con `(0, 0)` in alto a sinistra. Per `hex pointy` sono spostate le righe dispari (odd-r), per `hex flat` le colonne dispari (odd-q). Sono le coordinate più semplici da scrivere per un agente; la convenzione va documentata nella skill (M3).
- La griglia è calcolata **in Rust** e testata (centri, ingombro, celle adiacenti), come imposition e rettangolo inscritto.
- Errori: `manifest.grid` per i valori non validi, una griglia che esce dalla plancia, un'orientation in conflitto o un `cell` che non corrisponde a un componente (con `did you mean`).

### 3.2 Nei template

La griglia calcolata arriva in `ctx.grid`. La stdlib aggiunge:

| Funzione | Comportamento |
|---|---|
| `cell-center(grid, col, row)` | centro della cella, in coordinate del canvas (bleed incluso) |
| `cells(grid)` | tutte le celle, come array di `(col, row, x, y)` |
| `draw-grid(grid, stroke: ..., cell: none)` | disegna i contorni; se `cell` è una funzione `(col, row) => content`, ne posiziona il risultato al centro di ogni cella |

I nomi esatti si possono rifinire in implementazione. La semantica no: coordinate offset, centri nel sistema di riferimento del canvas, dimensione della cella uguale a `cell`.

### 3.3 Nello schema: `format: grid-cell`

Nuovo formato custom, ammesso solo nei componenti con `grid`. Il valore è un array `[col, row]` di due interi. `rtt check` verifica che sia dentro la griglia; altrimenti produce `data.grid-cell`, con i limiti nel suggerimento:

```
error[data.grid-cell]: `spaces.0.at`: [3, 12] is outside the grid
  --> game.yaml:31
  at: dungeon-map / dungeon-map / spaces.0.at
  hint: grid is 14 × 9: col 0..13, row 0..8
```

## 4. Geometria condivisa tra componenti

- Ogni template riceve `ctx.components`: un dizionario `id → (kind, shape, trim, bleed)` con **tutti** i componenti del progetto. Una plancia può dimensionare uno spazio sulle carte (`ctx.components.unit-card.trim`) o sui token senza ripetere numeri.
- `grid.cell: <id>` è la scorciatoia del manifest per il caso più comune: le celle della griglia hanno la stessa dimensione (ed eventualmente orientamento) di un token. Combinazioni ammesse:
  - `hex` ← token `hex` (eredita la distanza tra i lati e `orientation`);
  - `square` ← token `square` o `circle` (lato = lato o diametro).

  Altre combinazioni producono `manifest.grid`.
- Questa è la "relazione" minima tra componenti prevista da Decisioni §4: si legge la geometria, non c'è un grafo.

## 5. Montaggio della modalità `tiles`

- La faccia è disegnata una sola volta a dimensione piena e ritagliata in sezioni grandi quanto l'area stampabile, con `overlap` di sovrapposizione (Spec M2.5 §4).
- **Convenzione:** ogni sezione si sovrappone a quella a destra e a quella sotto. Su ogni sezione, la striscia che va **sotto** la vicina è segnata da un tratteggio leggero, con l'etichetta della sezione che la copre ("under B1"). Il bordo da rifilare è indicato da una linea tratteggiata con un simbolo di forbici.
- Ogni pagina riporta l'etichetta della sezione (A1, A2, … per riga e colonna) e i segni di registro agli angoli.
- **Pagina di montaggio**, prima delle sezioni: la miniatura della plancia con la griglia delle sezioni e le etichette, più la dimensione finale.
- Il testo stampato sui fogli è in inglese, come la CLI.

## 6. Esempio

`examples/sigilli` aggiunge `dungeon-map`: una plancia 500×350 mm, senza file di dati, con `fields` inline e una griglia `hex` con `cell: vortex`. Il template usa `draw-grid` e colora gli `spaces` dichiarati nei dati.

## 7. Criterio di uscita

- `rtt build examples/sigilli` produce le sezioni della plancia su A4, con sovrapposizione, etichette, segni di registro, strisce tratteggiate, linee di rifilo e la pagina di montaggio.
- Test geometrico: l'unione delle sezioni, al netto delle sovrapposizioni, copre l'intera plancia + bleed senza buchi.
- `mode: poster` produce una pagina singola della dimensione della plancia + bleed.
- Test della griglia: la dimensione delle celle è uguale alla distanza tra i lati del token `vortex`; i centri di celle adiacenti distano esattamente `cell`.
- `at: [3, 12]` su una griglia 14×9 produce `data.grid-cell` localizzato sulla riga del record.
- `grid.cell: vortx` produce `manifest.grid` con `did you mean \`vortex\``.
- `back` con `mode: tiles` produce `manifest.print`.
- Le linee di taglio dei token usano la doppia traccia (verifica visiva sui PNG).
- Build deterministico, CI verde, `Decisioni v0.1.md` aggiornato.

## 8. Fuori da M2.6

- **`render.low-resolution` → M3.** `art()` segnala i pixel effettivi alla dimensione stampata; sotto i 200 dpi scatta un avviso localizzato sul record. Vale per tutti i kind, ma pesa soprattutto sulle plance.
- `track` (percorsi di punteggio sul bordo), più griglie per plancia, griglie sulle carte o sui token: backlog, quando un prototipo reale ne ha bisogno.
- Forme da SVG, disposizione a nido d'ape degli esagoni, export SVG di taglio, export TTS (M4).
