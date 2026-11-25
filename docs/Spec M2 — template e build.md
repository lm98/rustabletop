# rustabletop — Spec M2: template e build

**Stato:** completata (PR #2, 23/11/2026). Il contratto della faccia di §1 resta valido; manifest, `ctx` e produzione sono stati generalizzati da "Spec M2.5 — modello dei componenti e token.md".
**Contesto:** vedi "Decisioni v0.1.md" (§3.4, §3.5, §5).

---

## 0. Stato del repo al momento della spec

- Repo: `lm98/rustabletop`. `main` conteneva M0; `m1-check` conteneva M1 (PR #1).
- M2 è stata sviluppata sul branch `m2-build`, creato da `m1-check`.
- Crate: `rtt-core` (manifest, `source` con posizioni per JSON pointer, `schema` con lint ed effective schema, `world` Typst, `diagnostics`), `rtt-cli` (`rtt check`, `rtt render` di basso livello).
- Typst fissato a `=0.15.1`. L'API cambia tra versioni minori: verificare sempre le firme nei sorgenti in `~/.cargo/registry`.
- Riferimenti: `spike/m0a` (imposition con bleed sensibile ai vicini, `fit-text`, icone in linea già corrette) ed `examples/sigilli`.
- Stile: codice autoesplicativo, niente commenti didattici. CI con `cargo fmt --check`, `clippy -D warnings`, test e `rtt check examples/sigilli`.

## 1. Contratto delle facce

Nel manifest ogni faccia indica un file Typst; il file esporta una **funzione con lo stesso nome della faccia**. Lo stesso file può esportare più facce.

```yaml
faces:
  front: templates/card.typ   # esporta `front(card, ctx)`
  back: templates/card.typ    # esporta `back(card, ctx)`
```

```typst
#import "@rtt/std:0.1.0": rich, fit, safe-area

#let front(card, ctx) = {
  rect(width: 100%, height: 100%, fill: red)
  safe-area(ctx, inset: 2mm)[#fit(rich(card.text))]
}
```

- `card`: record validato (dizionario con `id`, `qty` e i campi dello schema).
- `ctx`: `(trim: (width, height), bleed, component, face)`. Esteso in M2.5 con `kind`, `shape` e `item`.
- La funzione disegna su un'area `trim + 2·bleed`, con origine nell'angolo del bleed. Il framework impone dimensioni e clip. **Il template non imposta mai la pagina.**
- Il framework genera un **file principale virtuale** che importa le funzioni dei template, legge le istanze da un **JSON virtuale** servito dal `World` e le richiama. Una sola compilazione per build.
- Scartato: un documento per carta, perché richiederebbe N compilazioni e l'imposition dovrebbe comunque ricomporre le facce.
- Il check verifica che il file della faccia esista e che definisca la funzione.

## 2. Stdlib `@rtt/std:0.1.0`

Servita dal `World` come **pacchetto virtuale** (`VirtualRoot::Package` con namespace `rtt`), con i file incorporati nel binario (`include_str!`).

| Funzione | Comportamento |
|---|---|
| `rich(body)` | Sostituisce `:nome:` con l'icona dichiarata nel manifest. Icona impaginata con `place` dentro una box di altezza fissa, così non altera l'interlinea (correzione dello spike). |
| `fit(body, max: 9pt, min: 5.5pt)` | Riduce il testo fino a farlo stare nel contenitore (`layout` + `measure`). Se non ci sta nemmeno a `min`, emette `metadata((kind: "rtt.overflow", …))` con un'etichetta dedicata. |
| `safe-area(ctx, inset:)` | Riquadro interno al rifilo. |

**Overflow → diagnostica.** Dopo la compilazione, Rust interroga il documento per l'etichetta di overflow e produce:

```
warning[render.overflow]: `text` of unit-card/void-oracle does not fit at 5.5pt
  --> data/units.yaml:58
```

La diagnostica è localizzata sul record grazie alle posizioni conservate da M1.

## 3. Icone

```yaml
icons:
  fuoco: icons/fuoco.svg
  acqua: icons/acqua.svg
```

- Solo file SVG/PNG dichiarati nel manifest, a livello di progetto. Il check verifica che i file esistano.

## 4. Formati custom nello schema

| Formato | Controllo in `rtt check` |
|---|---|
| `format: rich-text` | Ogni token `:nome:` deve essere un'icona dichiarata. Altrimenti `data.unknown-icon`, con `did you mean`. |
| `format: asset` | Il percorso deve esistere nella root del progetto. Altrimenti `data.missing-asset`. Se il campo è opzionale e manca, il template mostra un segnaposto. |

## 5. Stampa e font

```yaml
print:
  paper: a4            # a4 | letter
  margin: 5mm
  crop-marks: true
  duplex: long-edge    # long-edge | short-edge | none
fonts: fonts/          # default: fonts/ se la cartella esiste
```

- Rust calcola la griglia (colonne e righe per pagina) e segnala un errore se nemmeno una carta con bleed sta nell'area stampabile.
- Imposition come **modulo Typst generato**: carte accostate al rifilo, bleed solo sui lati senza vicino occupato, crocini sulle linee di taglio, retri specchiati.
- `back` assente → nessuna pagina di retri.
- *(Superato da M2.5: `print` diviso tra progetto e componente.)*

## 6. `rtt build`

```
rtt build [PROJECT] [--json] [--out out/] [--ppi 300]
```

- Esegue `check`; se ci sono errori si ferma e li riporta.
- Produce `out/<game>-pnp.pdf` e `out/<component>/<id>-<face>.png`.
- Report JSON: `{ ok, artifacts, diagnostics }`, con gli overflow tra le diagnostiche.
- Gli errori dei template restano con codice `typst` e puntano al file `.typ`.

## 7. Criterio di uscita

- `examples/sigilli` si costruisce dal manifest con `rtt build`, senza un `pnp.typ` scritto a mano, e produce un PDF equivalente a quello dello spike (6 pagine A4, retri specchiati).
- `void-oracle` con un testo allungato produce un `render.overflow` che punta alla riga del record.
- `:fuco:` in un campo `rich-text` produce `data.unknown-icon` con `did you mean`.
- Test di integrazione per i tre punti; build deterministico (hash del PDF stabile tra due esecuzioni).

## 8. Fuori da M2

`rtt schema`, `rtt render <id>` come anteprima mirata e la skill sono M3; l'export TTS è M4.
