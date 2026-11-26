# Verifiche manuali di M3.1

Criteri di uscita di "Spec M3.1 — distribuzione.md" §7 che non si possono automatizzare. Vanno eseguiti dopo il merge di M3.1 e la pubblicazione di `0.1.0-alpha.2` (tag, binari, zip della skill, crates.io). Ogni esito va annotato qui, con data e versione.

| # | Verifica | Come | Esito |
|---|---|---|---|
| 1 | binstall senza compilare | `cargo binstall rustabletop --version 0.1.0-alpha.2` su Linux x86_64, macOS arm64 e Windows x86_64: scarica l'archivio della release e non compila | da fare |
| 2 | `install.sh` con versione esplicita | `curl -fsSL …/install.sh \| RUSTABLETOP_VERSION=v0.1.0-alpha.2 sh` in una shell senza `~/.local/bin` nel `PATH`: installa la pre-release e stampa la riga `export PATH=…` | da fare |
| 3 | Zip accettato da claude.ai | `rustabletop skill export`, poi caricare `rustabletop-skill-0.1.0-alpha.2.zip` in Customize > Skills | da fare |
| 4 | Chat di claude.ai | Chat nuova con solo la skill caricata, brief: "Create a printable 10-card deck for a simple trick-taking game". Claude installa il binario alla versione della skill, costruisce il progetto e consegna il PDF, senza interventi | da fare |
| 5 | Claude Code in sessione nuova | Skill sincronizzata dall'account oppure `rustabletop skill install --dir ~`; uno dei tre brief di M3 (`../m3/`) superato senza interventi | da fare |

Se la verifica 4 fallisce perché la sandbox non raggiunge gli asset delle release di GitHub, si attiva il ripiego PyPI (Spec M3.1 §6). Qualsiasi altro fallimento va descritto in un file accanto a questo, come per `../m3/`.

Il protocollo di osservazione (cosa conta come intervento, cosa annotare) è quello di `../m3/README.md`.
