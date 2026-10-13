# Planner Settimanale V0.4

Planner desktop offline in italiano. Apre la settimana corrente; clicca **Scrivi qui** in un giorno e digita. `Invio` salva, `Maiusc+Invio` aggiunge una riga, `Esc` annulla. Le note sotto il planner sono specifiche della settimana. Le voci vengono salvate in SQLite nella cartella dati dell'applicazione.

## Requisiti

- Node.js 20 o successivo e npm.
- Rust e Cargo (toolchain `stable-msvc` su Windows).
- Windows: Microsoft C++ Build Tools e WebView2, come richiesto da Tauri 2.

## Sviluppo

```powershell
npm install
npm run tauri dev
```

## Build Windows

```powershell
npm install
npm run tauri build
```

L'installer `.exe` è generato in `src-tauri/target/release/bundle/nsis/`. Include l'installer offline di WebView2, quindi si può installare anche su un PC senza connessione; la dimensione del pacchetto aumenta. `npm run build` compila soltanto il frontend. Il planner resta utilizzabile offline; il controllo degli aggiornamenti richiede Internet.

## Comandi

- Frecce in alto: settimana precedente o successiva; **Oggi**: settimana corrente.
- `Ctrl+T`: settimana corrente; `Alt+←` e `Alt+→`: navigazione.
- Click su una voce: modifica; click destro: completa, sposta, duplica o elimina.
- Dal click destro puoi anche impostare un orario: la voce viene collocata più in basso nel giorno in base all'ora scelta. Cliccando una riga vuota, la nuova voce resta sulla riga scelta.
- **Esporta dati** salva tutte le settimane in un file JSON leggibile, incluse voci, posizioni, orari, checkbox e note. **File → Importa dati…** apre un JSON esportato, ne mostra il contenuto e unisce voci e note senza sostituire i dati esistenti. Prima di importare crea automaticamente un backup SQLite nella cartella dati dell’app.
- **Aiuto → Controlla aggiornamenti…** verifica le release firmate di `ErBrandy90/Planner-personale` e consente di installarle dall’app. Il comando richiede una release GitHub pubblica contenente `latest.json`, l’installer e il file `.sig`.
- `[] Testo`: attività con checkbox. Un orario iniziale come `18:30` viene registrato ma non cambia la posizione della voce.

Il salvataggio delle voci avviene al momento della conferma o quando l'editor perde il focus. Le note vengono salvate dopo 250 ms di pausa, alla perdita del focus e al cambio settimana. SQLite usa WAL e `synchronous=FULL`.

## Assistenti AI tramite MCP

Il pacchetto `.dxt` del connettore (pubblicato tra gli asset delle [Releases](https://github.com/ErBrandy90/Planner-personale/releases); in locale si trova in `mcp-extension/`, escluso da git) collega Claude Desktop al database locale. Permette di leggere, creare e completare attività e di aggiungere alle note gli impegni ancora senza una data precisa. Il protocollo MCP è aperto e lo stesso server `planner_mcp.exe` può essere collegato in futuro ad altri client MCP, senza dipendere da Claude. Le modifiche esterne compaiono nel planner aperto entro circa tre secondi.

Consulta [`docs/MCP_CLAUDE.md`](docs/MCP_CLAUDE.md) per l'installazione e gli esempi d'uso.

## Pubblicare una versione per l’updater

**Numero di versione:** si cambia in un solo comando, `npm run versione -- 0.4.1`.
- La fonte è `package.json`. `tauri.conf.json` la legge da lì e l'interfaccia la chiede all'app (`getVersion()`).
- Lo script allinea anche `Cargo.toml`, `Cargo.lock`, `package-lock.json` e il manifest del connettore.
- `npm run versione` senza argomenti controlla che tutto coincida, e lo fa anche la CI.
- Restano a mano il changelog in `src/App.tsx` e la documentazione.

Ogni build produce l’installer NSIS `.exe`. Per produrre anche la firma `.sig`, imposta `TAURI_SIGNING_PRIVATE_KEY` al percorso della chiave privata conservata in `.signing/planner.key` e lancia `npm run tauri build`. **Non pubblicare la chiave privata** e conservarne una copia sicura: senza quella chiave le versioni già installate non potranno verificare aggiornamenti futuri. La cartella `.signing/` è esclusa da Git.

Comando per le build future (verificato):

```bash
# da Git Bash, nella cartella del progetto
TAURI_SIGNING_PRIVATE_KEY="$(cat .signing/planner.key)" TAURI_SIGNING_PRIVATE_KEY_PASSWORD= npm run tauri build
```

Attenzione, due trappole verificate il 13/10/2026:

- **Non usare Windows PowerShell 5.1:** `$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''` *cancella* la variabile invece di lasciarla vuota. Tauri allora si ferma ad aspettare una password che nessuno scrive.
- **Passare il contenuto della chiave, non un percorso relativo:** `.signing/planner.key` viene letto come se fosse la chiave stessa, e la firma fallisce con "Invalid symbol 46".

Per ogni versione da distribuire, carica sulla release GitHub l’installer `.exe`, il suo `.sig` e un `latest.json` con versione, note, URL HTTPS dell’installer e contenuto testuale della firma per `windows-x86_64`. Il repository deve rendere questi file scaricabili senza autenticazione. Una build locale crea l’`.exe` anche quando non viene pubblicata come aggiornamento.

La release 0.4.0 è la prima che contiene il controllo degli aggiornamenti: l’aggiornamento da 0.3.1 a 0.4.0 richiede quindi l’installazione manuale dell’`.exe` 0.4.0. Le release successive potranno essere installate dal menu.

## Documentazione

- [`STATO_LAVORI.md`](STATO_LAVORI.md): dove sta tutto, comandi, stato, cronologia e punti aperti. **Va aggiornato, committato e pushato a ogni modifica.**
- [`docs/DOCUMENTAZIONE_PROGETTO_E_ROADMAP.md`](docs/DOCUMENTAZIONE_PROGETTO_E_ROADMAP.md): funzioni, architettura, database, versioni e roadmap.
- [`docs/MCP_CLAUDE.md`](docs/MCP_CLAUDE.md): installazione del connettore in Claude Desktop.

## Test

```powershell
npm run build                                                   # TypeScript + Vite
cargo test --locked --manifest-path src-tauri/Cargo.toml        # test Rust (importazione, note, connettore)
cargo build --release --locked --bin planner_mcp --manifest-path src-tauri/Cargo.toml
python tests/test_mcp.py                                        # test end-to-end del connettore MCP
```

Gli stessi controlli girano su GitHub Actions a ogni push (`.github/workflows/test.yml`).
