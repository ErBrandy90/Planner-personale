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
| Chiave di firma updater | `.signing/planner.key` (esclusa da git). **Va conservata anche nel gestore di password**: se si perde, le versioni installate non accettano più aggiornamenti |
| Installer e firme generati | `release-artifacts/<versione>/` (esclusa da git) |
| Connettore MCP per Claude Desktop | `mcp-extension/` (`manifest.json` in git; `.dxt` ed `.exe` esclusi, si pubblicano come asset delle Releases) |
| Database dell'app | `%APPDATA%\it.planner.settimanale\planner.sqlite3`, backup in `backups/` nella stessa cartella |

Documentazione nel repo: `README.md` (uso, build, pubblicazione), `docs/DOCUMENTAZIONE_PROGETTO_E_ROADMAP.md` (funzioni, architettura, database, roadmap), `docs/MCP_CLAUDE.md` (connettore).

## 2. Comandi utili

```powershell
npm install
npm run tauri dev                                               # app in sviluppo
npm run build                                                   # frontend: TypeScript + Vite
cargo test --locked --manifest-path src-tauri/Cargo.toml        # 5 test Rust (importazione)
cargo build --release --locked --bin planner_mcp --manifest-path src-tauri/Cargo.toml
python tests/test_mcp.py                                        # test end-to-end del connettore MCP

# Build firmata (installer + .sig per l'updater)
$env:TAURI_SIGNING_PRIVATE_KEY = (Resolve-Path '.signing\planner.key').Path
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
npm run tauri build
```

## 3. Stato

Versione **0.4.0**, funzionante e in uso.

- Planner settimanale offline (Tauri 2 + React + Rust + SQLite), installer NSIS offline con WebView2.
- Voci, checkbox, orari, note settimanali, temi, esportazione JSON.
- **0.4.0**: importazione JSON con anteprima e backup; **Aiuto → Controlla aggiornamenti** con pacchetti firmati.
- Connettore MCP (`planner_mcp.exe`, libreria `rmcp`) per Claude Desktop: `list_tasks`, `create_task`, `set_task_completed`, `add_weekly_notes`.
- Test: 5 test Rust sull'importazione + test MCP end-to-end in Python, su GitHub Actions (Windows).

## 4. Cronologia delle modifiche

| Commit | Cosa |
|---|---|
| (primo commit) | Prima pubblicazione su GitHub della 0.4.0: `.gitignore` e `.gitattributes`, documentazione spostata in `docs/` e aggiornata alla 0.4.0, `STATO_LAVORI.md`, workflow di test |

## 5. Da verificare / aperto

Bug e debolezze emersi dall'audit del 13/10/2026. **Da correggere uno per volta**, ognuno con il suo commit.

1. **Note sovrascritte** (`src/App.tsx`, `syncExternal` + `WeeklyNotes`): se il connettore MCP aggiunge note mentre si sta scrivendo nel campo note, il salvataggio successivo dell'utente cancella le note aggiunte dal connettore.
2. **Note a rischio in `add_weekly_notes`** (`src-tauri/src/bin/planner_mcp.rs`): `query_row(...).unwrap_or_default()` tratta un errore di lettura come "note vuote" e poi riscrive la riga senza le note esistenti. Va usato `.optional()?`.
3. **Errori MCP come testo**: gli errori tornano come testo "Errore: …" e non come risultato con `isError`, quindi il client non capisce che l'operazione è fallita.
4. **Versione scritta a mano in 5 punti** (`package.json`, `Cargo.toml`, `tauri.conf.json`, `App.tsx` → `appVersion`, documentazione). In `App.tsx` usare `getVersion()` di `@tauri-apps/api/app`; regola generale: versione in un solo punto.
5. **`csp: null`** in `tauri.conf.json`: nessuna Content Security Policy.
6. **Chiave di firma dell'updater** presente solo sul PC: farne una copia sicura nel gestore di password e valutare di proteggerla meglio.
7. **Codice su righe lunghissime** (Rust e TSX): passare `cargo fmt` ed eventualmente Prettier.
8. **Migrazioni del DB ad hoc**, senza `PRAGMA user_version`.
9. **Backup senza rotazione**: la cartella `backups/` cresce senza limiti.
10. **Test**: `tests/test_mcp.py` funziona solo su Windows (percorso `.exe` fisso) e il server MCP non ha test Rust.
11. **Versione del connettore**: `mcp-extension/manifest.json` dice 0.3.1, ma in cartella ci sono solo i `.dxt` 0.1.0, 0.2.0 e 0.3.0. Decidere quale `.dxt` pubblicare (o rigenerarlo).

## 6. Decisioni prese (da non rimettere in discussione)

- Il planner resta utilizzabile offline; SQLite è la copia locale principale.
- Le integrazioni AI passano da MCP; le operazioni ambigue non inventano date o orari.
- Aggiornamenti tramite GitHub Releases su repo pubblica, con pacchetti firmati; nella app solo la chiave pubblica.
- Binari (`.exe`, `.dxt`, installer) fuori dalla storia git: si pubblicano come asset delle Releases.
- Altre decisioni di progetto: `docs/DOCUMENTAZIONE_PROGETTO_E_ROADMAP.md` §17.
