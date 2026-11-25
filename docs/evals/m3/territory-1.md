# territory — esecuzione 1

- **Data:** 25/11/2026. **Versione:** `rustabletop 0.1.0-alpha.1` da `v0.1.0-alpha.1-test.2` via `install.sh` (URL del branch), skill da `rustabletop skill install`.
- **Esecutore:** sotto-agente in una cartella pulita, non una sessione nuova (approssimazione concordata), messaggio iniziale = solo il brief.
- **Esito:** superato (in attesa della verifica visiva umana).
- **Interventi:** nessuno.
- **Risultato:** "Six Realms": plancia 350×250 con griglia hex 10×8 presa dal token, 6 terreni con icone, santuari e fortezze; 2 set da 8 gettoni hex 25 mm (Keep, Knight, Soldier, Scout) in `fold`; 12 carte obiettivo in duplex. PDF di 8 pagine A4 (carte 4, gettoni 1, montaggio + 2 sezioni).
- **`rustabletop check`:** `ok: six-realms (objective: 12 instances, 12 copies; unit: 8 instances, 16 copies; board: 1 instances, 1 copies), 0 warning(s)` (riverificato).
- **`rustabletop build`:** `ok: 42 artifact(s), 0 warning(s)` (riverificato).
- **Verifica visiva (preliminare, mia):** coerente col brief.
- **Diagnostiche incontrate:** nessuna. Ha usato anche `build --draft`.
- **Osservazioni dell'agente → correzioni:**
  - **Problema reale:** una plancia 360×250 su A4 era divisa in 3 sezioni di cui l'ultima larga 4 mm, e `printing.md` diceva "2×2". L'agente ha trovato la soglia (355 mm → 2 sezioni) per tentativi → `spans` divide ora la plancia in sezioni uguali (stesso numero di pagine, stessa sovrapposizione, test `sections_share_the_board_evenly`); `printing.md` riporta la dimensione utile di una sezione e esempi corretti; la plancia del progetto iniziale passa a 350×250 (2 sezioni).
  - `build --draft` sovrascriveva i PNG definitivi → suffisso `-draft`.
  - Default di `--face` e `render <component>` per componenti a istanza singola non documentati → `SKILL.md`.
  - Posizionamento di `cell-shape`, import di file `.typ` vicini, mappa cella → valore, `data` su una board, asse del `size` esagonale → `templates.md` e `manifest.md`.
