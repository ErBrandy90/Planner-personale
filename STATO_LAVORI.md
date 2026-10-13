# Planner Settimanale · stato dei lavori

File di servizio per riprendere il lavoro in una nuova sessione.
**Regola: a ogni modifica del progetto questo file va aggiornato, committato e pushato su GitHub.**

Ultimo aggiornamento: **13 ottobre 2026**

---

## 1. Dove sta tutto

| Cosa | Dove |
|---|---|
| Codice | GitHub **pubblico** [ErBrandy90/Planner-personale](https://github.com/ErBrandy90/Planner-personale), ramo `main`. È pubblico perché l'updater scarica `latest.json` dalle Releases senza autenticazione: **mai segreti nella repo** |
| Aggiornamenti | GitHub Releases: `latest.json`, installer `.exe` e firma `.sig` per ogni versione; endpoint in `src-tauri/tauri.conf.json` |
| Chiave di firma updater | `.signing/planner.key` (esclusa da git), con una copia nel gestore di password: se si perde, le versioni installate non accettano più aggiornamenti |
| Installer e firme generati | `release-artifacts/<versione>/` (esclusa da git) |
| Connettore MCP per Claude Desktop | `mcp-extension/` (`manifest.json` in git; `.dxt` ed `.exe` esclusi, si pubblicano come asset delle Releases) |
| Database dell'app | `%APPDATA%\it.planner.settimanale\planner.sqlite3`, backup in `backups/` nella stessa cartella |

Documentazione nel repo: `README.md` (uso, build, pubblicazione), `docs/DOCUMENTAZIONE_PROGETTO_E_ROADMAP.md` (funzioni, architettura, database, roadmap), `docs/MCP_CLAUDE.md` (connettore).

## 2. Comandi utili

```powershell
npm install
npm run tauri dev                                               # app in sviluppo
npm run build                                                   # frontend: TypeScript + Vite
npm run versione -- 0.4.1                                       # nuova versione, allineata ovunque (senza numero: solo controllo)
npm run format                                                  # formatta TypeScript/CSS (Prettier) e Rust (cargo fmt); format:check per controllare
cargo test --locked --manifest-path src-tauri/Cargo.toml        # 17 test Rust (importazione, note, avvio database, connettore)
cargo build --release --locked --bin planner_mcp --manifest-path src-tauri/Cargo.toml
python tests/test_mcp.py                                        # test end-to-end del connettore MCP
```

Build firmata (installer + `.sig` per l'updater), **da Git Bash**. In PowerShell 5.1 la password vuota non funziona: vedi il README.

```bash
TAURI_SIGNING_PRIVATE_KEY="$(cat .signing/planner.key)" TAURI_SIGNING_PRIVATE_KEY_PASSWORD= npm run tauri build
```

Per pubblicare una release:
1. `npm run versione -- X.Y.Z`, aggiornare il changelog in `App.tsx` e la documentazione.
2. Fare la build firmata.
3. Copiare installer e `.sig` in `release-artifacts/X.Y.Z/`.
4. Rigenerare il `.dxt` se il connettore è cambiato.
5. Scrivere `latest.json`, con l'URL che usa `Planner.Settimanale_…` (il punto al posto dello spazio).
6. `gh release create vX.Y.Z … --latest`.

## 3. Stato

Versione **0.4.1** (13/10/2026): correzioni dei bug 1-5 di questa lista.

- Planner settimanale offline (Tauri 2 + React + Rust + SQLite), installer NSIS offline con WebView2.
- Voci, checkbox, orari, note settimanali, temi, esportazione JSON.
- **0.4.0**: importazione JSON con anteprima e backup; **Aiuto → Controlla aggiornamenti** con pacchetti firmati.
- Connettore MCP (`planner_mcp.exe`, libreria `rmcp`) per Claude Desktop: `list_tasks`, `create_task`, `set_task_completed`, `add_weekly_notes`.
- Test: 17 test Rust (importazione, unione delle note, avvio del database, connettore) + test MCP end-to-end in Python, su GitHub Actions (Windows).
- **Release [v0.4.1](https://github.com/ErBrandy90/Planner-personale/releases/tag/v0.4.1) pubblicata il 13/10/2026**: installer, `.sig`, `latest.json` e `.dxt` 0.4.1.
  - **Aggiornamento dall'app provato con successo** sul PC (0.4.0 → 0.4.1): l'app ha fatto il backup del database, si è aggiornata e ora mostra 0.4.1.
  - Le copie pubblicate stanno in `release-artifacts/0.4.1/`. In `src-tauri/target/…/nsis` può esserci una build successiva, non pubblicata.
- Release [v0.4.0](https://github.com/ErBrandy90/Planner-personale/releases/tag/v0.4.0) pubblicata il 13/10/2026: installer, `.sig`, `latest.json` e `.dxt` 0.4.0. L'endpoint `releases/latest/download/latest.json` risponde.
- **Attenzione per le prossime release**: GitHub sostituisce gli spazi dei nomi file con punti (`Planner Settimanale_…exe` → `Planner.Settimanale_…exe`). L'`url` in `latest.json` deve usare il nome con il punto.
- CI GitHub Actions verde (prima esecuzione circa 8 minuti).

## 4. Cronologia delle modifiche

| Commit | Cosa |
|---|---|
| `b1815a2` | Prima pubblicazione su GitHub della 0.4.0: `.gitignore` e `.gitattributes`, documentazione spostata in `docs/` e aggiornata alla 0.4.0, `STATO_LAVORI.md`, workflow di test |
| `947c249` | Connettore MCP portato alla 0.4.0: `manifest.json` 0.4.0 e nuovo pacchetto `Planner-Settimanale-0.4.0.dxt`, pubblicato nella release v0.4.0 |
| `1688629` | Registra la pubblicazione della release v0.4.0 e il primo esito verde della CI |
| `6257732` | Corregge le note sovrascritte: unione delle righe aggiunte dal connettore MCP, salvataggi in fila, cursore fermo |
| `c05127b` | Connettore: un errore di lettura delle note non le cancella più (`add_weekly_notes`), transazione esclusiva, 3 test |
| `32b8260` | Connettore: gli errori arrivano al client come errori veri (`isError`), con test |
| `884a8ca` | Versione in un solo punto (`package.json`): `tauri.conf.json` la legge da lì, `getVersion()` nell'interfaccia, `scripts/versione.mjs` per il resto, controllo in CI |
| `0916d9e` | Content Security Policy al posto di `csp: null`, con `devCsp` per lo sviluppo |
| `70385c8` | Versione 0.4.1: changelog, documentazione, manifest del connettore (release v0.4.1 pubblicata) |
| `1a645ef` | Comando di build firmata verificato (Git Bash) al posto dell'esempio PowerShell, che si bloccava; aggiornamento 0.4.0 → 0.4.1 provato |
| `c14f602` | Correzioni 0.4.1 verificate nell'app vera: note con Claude Desktop e CSP |
| `decf991` | Formattazione: `cargo fmt` e Prettier, `run()` riscritta in modo leggibile con test sull'avvio del database, controllo del formato in CI |
| `e08e934` | `.git-blame-ignore-revs` per il commit di formattazione |
| (questo commit) | Chiave di firma salvata nel gestore di password; migrazioni, rotazione dei backup e test multipiattaforma spostati tra le implementazioni future |

## 5. Da verificare / aperto

Bug e debolezze emersi dall'audit del 13/10/2026. **Da correggere uno per volta**, ognuno con il suo commit.

1. ~~**Note sovrascritte**~~ **corretto il 13/10/2026**:
   - `save_notes` unisce in fondo le righe aggiunte da fuori invece di cancellarle (`src-tauri/src/notes.rs`, 6 test);
   - il frontend tiene la "base" per ogni settimana, salva le note una alla volta e non sostituisce mai testo non ancora salvato;
   - provato in un banco di prova con il connettore simulato: scrittura durante l'aggiunta, salvataggio lento, campo fermo, riga cancellata.
   
   **Verificato nell'app vera** (0.4.1, con Claude Desktop e il connettore 0.4.1) il 13/10/2026.
2. ~~**Note a rischio in `add_weekly_notes`**~~ **corretto il 13/10/2026**: la lettura usa `.optional()?`, quindi un errore ferma l'operazione senza scrivere nulla, e la transazione è esclusiva (`Immediate`) come quella dell'app. La logica è in `append_weekly_notes`, con 3 test Rust. Incluso nella 0.4.1.
3. ~~**Errori MCP come testo**~~ **corretto il 13/10/2026**: gli strumenti restituiscono `Result<String, String>`, quindi un errore arriva al client come risultato con `isError: true`. Verificato con un test Rust e nel test end-to-end (attività inesistente, data non valida). Incluso nella 0.4.1.
4. ~~**Versione scritta a mano in 5 punti**~~ **corretto il 13/10/2026**:
   - la fonte unica è `package.json`;
   - `tauri.conf.json` usa `"version": "../package.json"` e `App.tsx` usa `getVersion()`;
   - `npm run versione -- X.Y.Z` allinea `Cargo.toml`, `Cargo.lock`, `package-lock.json` e `mcp-extension/manifest.json`;
   - `npm run versione` controlla l'allineamento, anche in CI.
   
   Provato portando la versione a 0.4.9 (l'eseguibile compilato riportava 0.4.9) e poi di nuovo a 0.4.0. Restano a mano il changelog in `App.tsx` e la documentazione.
5. ~~**`csp: null`**~~ **corretto il 13/10/2026**:
   - CSP stretta (`'self'` + IPC di Tauri, niente inline);
   - `devCsp` separata per lo sviluppo (stili inline e websocket di Vite);
   - provata nel browser con l'interfaccia vera: nessuna violazione, uno script e uno stile inline di prova correttamente bloccati, e anche lo sviluppo con Vite funziona.
   
   **Verificato nella finestra Tauri vera** (installer 0.4.1) il 13/10/2026: l'app funziona normalmente con la CSP attiva.
6. ~~**Chiave di firma dell'updater**~~ **fatto il 13/10/2026**: copia della chiave salvata da Marco nel gestore di password.
7. ~~**Codice su righe lunghissime**~~ **corretto il 13/10/2026**:
   - Rust formattato con `cargo fmt`; TypeScript e CSS con Prettier (versione fissata, apici singoli, 120 caratteri, solo `src/` e `scripts/`);
   - `run()` in `lib.rs` riscritta a mano: schema SQL in `SCHEMA`, avvio del database in `prepare_database`, con 2 test;
   - `npm run format` riformatta, `npm run format:check` controlla, anche in CI;
   - restano più lunghe di 150 caratteri solo alcune stringhe SQL e le descrizioni degli strumenti MCP, che `rustfmt` non spezza.
11. ~~**Versione del connettore**~~ risolto il 13/10/2026: rigenerato `Planner-Settimanale-0.4.0.dxt` dal `planner_mcp.exe` della 0.4.0. Il `.dxt` va rigenerato a ogni release in cui cambia il server MCP (zip di `icon.png`, `manifest.json`, `server/planner_mcp.exe`).

## 5b. Implementazioni future

Punti rimandati, da affrontare uno per volta, ognuno con il suo commit e la sua release:

- **Migrazioni del DB ad hoc**, senza `PRAGMA user_version`.
- **Backup senza rotazione**: la cartella `backups/` cresce senza limiti.
- **Test**: `tests/test_mcp.py` funziona solo su Windows (percorso `.exe` fisso). Il server MCP ha 4 test Rust (note e gestione degli errori).

## 6. Decisioni prese (da non rimettere in discussione)

- Il planner resta utilizzabile offline; SQLite è la copia locale principale.
- Le integrazioni AI passano da MCP; le operazioni ambigue non inventano date o orari.
- Aggiornamenti tramite GitHub Releases su repo pubblica, con pacchetti firmati; nella app solo la chiave pubblica.
- Binari (`.exe`, `.dxt`, installer) fuori dalla storia git: si pubblicano come asset delle Releases.
- Numero di versione solo in `package.json`; gli altri file si allineano con `npm run versione -- X.Y.Z`, mai a mano.
- Altre decisioni di progetto: `docs/DOCUMENTAZIONE_PROGETTO_E_ROADMAP.md` §17.
