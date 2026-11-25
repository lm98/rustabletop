# push-your-luck — esecuzione 1

- **Data:** 25/11/2026. **Versione:** `rustabletop 0.1.0-alpha.1` da `v0.1.0-alpha.1-test.2` via `install.sh` (URL del branch), skill da `rustabletop skill install`.
- **Esecutore:** sotto-agente in una cartella pulita, non una sessione nuova (approssimazione concordata), messaggio iniziale = solo il brief.
- **Esito:** superato; verifica visiva umana approvata (25/11/2026).
- **Interventi:** nessuno.
- **Risultato:** "Lucky Dive", 30 carte in 3 semi (Pearl, Coral, Gold) con valori 1-10 e una regola per seme, 12 monete fronte/retro (1 / 3 monete) `circle` in `fold`. PDF di 9 pagine A4: 4 fogli di fronti + 4 di retri in duplex con crocini, 1 foglio di monete da piegare.
- **`rustabletop check`:** `ok: lucky-dive (card: 30 instances, 30 copies; coin: 1 instances, 12 copies), 0 warning(s)` (riverificato).
- **`rustabletop build`:** `ok: 63 artifact(s), 0 warning(s)` (riverificato).
- **Verifica visiva (preliminare, mia):** fronti, retri e monete leggibili e coerenti col brief; fogli corretti.
- **Diagnostiche incontrate:** nessuna. 12 comandi `rustabletop`, tutti puliti; un problema di impaginazione (riquadro del testo troppo alto) trovato guardando il render.
- **Osservazioni dell'agente → correzioni:**
  - `render --draft` sovrascriveva i PNG definitivi → i PNG in bozza finiscono in `-draft.png`.
  - Icone colorate anche in bozza, token `circle` che sul foglio sembrano quadrati, effetto di `radius` sulle carte non spiegato → note in `templates.md` e `manifest.md`.
  - Default di `--face` poco chiaro → `SKILL.md` riscritto.
  - Nessun modo di vedere un foglio impaginato con `rustabletop` (ha usato `pdftoppm`) → proposta, fuori da M3.
  - Niente opzione "progetto minimo" in `new` → nessuna modifica: cancellare i file dell'esempio è parte del ciclo previsto.
