# dungeon — esecuzione 1

- **Data:** 25/11/2026. **Versione:** `rustabletop 0.1.0-alpha.1` da `v0.1.0-alpha.1-test.2` via `install.sh` (URL del branch), skill da `rustabletop skill install`.
- **Esecutore:** sotto-agente in una cartella pulita, non una sessione nuova (approssimazione concordata), messaggio iniziale = solo il brief.
- **Esito:** superato (in attesa della verifica visiva umana).
- **Interventi:** nessuno.
- **Risultato:** "Depths of Ashmoor": 20 carte stanza (6 tipi, pericolo, testo con icone, illustrazione segnaposto con `art`), 10 gettoni salute `circle` in `fold`, plancia 280×230 con traccia del turno a 15 caselle disegnata dal template, 10 spazi per la salute e 3 spazi carta dimensionati da `ctx.components`. PDF di 10 pagine A4.
- **`rustabletop check`:** `ok: depths-of-ashmoor (room: 20 instances, 20 copies; health: 1 instances, 10 copies; board: 1 instances, 1 copies), 0 warning(s)` (riverificato).
- **`rustabletop build`:** `ok: 44 artifact(s), 0 warning(s)` (riverificato).
- **Verifica visiva (preliminare, mia):** coerente col brief.
- **Diagnostiche incontrate:** nessuna. Due problemi trovati solo guardando i render: spazi della salute che non stavano accanto alle carte (calcolo), cuore rosso su gettone rosso.
- **Osservazioni dell'agente → correzioni:**
  - Default di `--face`, `render <component>` a istanza singola, `artifacts` in `--json` → `SKILL.md`.
  - Icone che non si ricolorano → nota in `templates.md` (seconda icona chiara).
  - Sezioni che tagliano il contenuto → nota in `printing.md` (la sovrapposizione è stampata su entrambe).
  - Nessuna diagnostica per elementi sovrapposti o fuori dalla plancia, nessun conteggio di pagine per componente nel report di `build` → proposte, fuori da M3.
