# Valutazione M3 — protocollo operativo

Spec M3 §9 e §12.1. Un file `<brief>-<n>.md` per ogni esecuzione (`push-your-luck-1.md`, …).

## Preparazione della sessione (nuova, senza il repo)

```sh
curl -fsSL https://raw.githubusercontent.com/lm98/rustabletop/main/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
mkdir eval && cd eval
rustabletop skill install          # .claude/skills/rustabletop/, caricata all'avvio della sessione
```

Poi si apre la sessione Claude Code in `eval/`. La skill deve esserci **prima** del brief, altrimenti
l'agente non sa che rustabletop esiste; quando l'agente esegue `rustabletop new <gioco>`, il progetto
riceve la propria copia in `<gioco>/.claude/skills/rustabletop/`. Nessuna copia manuale (§12.1).

## Messaggi iniziali (esattamente questi, nient'altro)

1. **push-your-luck:** "Create a printable push-your-luck card game: 30 cards in 3 suits with values 1-10, and 12 double-sided coin tokens. No board."
2. **territory:** "Create a printable territory-control game: a 10×8 hex board with 6 terrain types, two sets of 8 hex tokens (one per player) and 12 objective cards."
3. **dungeon:** "Create a printable solo dungeon crawl: 20 room cards with placeholder illustrations, 10 health tokens, and a board with a turn track drawn by the template."

## Scheda di ogni esecuzione

```markdown
# <brief> — esecuzione <n>

- Data, versione (`rustabletop --version`), modello
- Esito: superato / fallito
- Interventi: nessuno / elenco dei messaggi umani dopo il brief
- `rustabletop check`: errori finali (0)
- `rustabletop build`: avvisi finali (0)
- Verifica visiva (umana): PDF e PNG coerenti col brief? note
- Diagnostiche incontrate durante il lavoro: codici e quante volte
- Correzioni apportate a skill / progetto iniziale / diagnostiche dopo questa esecuzione
```
