use super::{lock, Database};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::collections::HashSet;
use tauri::State;

fn key(line: &str) -> String {
    line.trim().to_lowercase()
}

/// Unisce le note salvate dall'app con le modifiche arrivate da fuori (connettore MCP, importazione).
/// `base` è il testo che l'app aveva letto dal database, `current` quello che c'è adesso, `mine` quello scritto nell'app.
/// Il testo dell'app resta com'è; in fondo si aggiungono le righe comparse nel database dopo `base` e non già presenti.
pub(crate) fn merge_notes(base: &str, current: &str, mine: &str) -> String {
    if current == base || current == mine {
        return mine.to_string();
    }
    let known: HashSet<String> = base.lines().chain(mine.lines()).map(key).collect();
    let mut seen = HashSet::new();
    let added: Vec<&str> = current
        .lines()
        .filter(|line| {
            !line.trim().is_empty() && !known.contains(&key(line)) && seen.insert(key(line))
        })
        .collect();
    if added.is_empty() {
        return mine.to_string();
    }
    let mine = mine.trim_end();
    if mine.is_empty() {
        added.join("\n")
    } else {
        format!("{mine}\n{}", added.join("\n"))
    }
}

/// Salva le note in una transazione esclusiva, così il connettore MCP non può scrivere a metà. Restituisce il testo salvato.
pub(crate) fn save(
    c: &mut Connection,
    week_start: &str,
    content: &str,
    base: &str,
) -> Result<String, String> {
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let current: String = tx
        .query_row(
            "SELECT content FROM weekly_notes WHERE week_start=?1",
            [week_start],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let value = merge_notes(base, &current, content);
    tx.execute("INSERT INTO weekly_notes(week_start,content) VALUES(?1,?2) ON CONFLICT(week_start) DO UPDATE SET content=excluded.content", params![week_start, value])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(value)
}

#[tauri::command]
pub(crate) fn save_notes(
    db: State<Database>,
    week_start: String,
    content: String,
    base: String,
) -> Result<String, String> {
    let mut c = lock(&db)?;
    save(&mut c, &week_start, &content, &base)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn connection() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE weekly_notes(week_start TEXT PRIMARY KEY,content TEXT NOT NULL);",
        )
        .unwrap();
        c
    }
    fn stored(c: &Connection) -> String {
        c.query_row("SELECT content FROM weekly_notes", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn senza_modifiche_esterne_salva_il_testo_dell_app() {
        assert_eq!(
            merge_notes("Spesa", "Spesa", "Spesa e farmacia"),
            "Spesa e farmacia"
        );
    }
    #[test]
    fn conserva_le_righe_aggiunte_dal_connettore_mentre_si_scrive() {
        let merged = merge_notes(
            "Spesa",
            "Spesa\n• Chiamare il commercialista",
            "Spesa e farmacia",
        );
        assert_eq!(merged, "Spesa e farmacia\n• Chiamare il commercialista");
    }
    #[test]
    fn non_duplica_righe_gia_scritte_nell_app() {
        assert_eq!(
            merge_notes("", "• Dentista", "Note\n• dentista"),
            "Note\n• dentista"
        );
    }
    #[test]
    fn rispetta_le_cancellazioni_fatte_nell_app() {
        // La riga "Vecchia" era già nella base: se l'utente la toglie non deve ricomparire.
        assert_eq!(
            merge_notes("Vecchia\nAltro", "Vecchia\nAltro\n• Nuova", "Altro"),
            "Altro\n• Nuova"
        );
    }
    #[test]
    fn note_vuote_nell_app_ricevono_solo_le_aggiunte() {
        assert_eq!(merge_notes("", "• Uno\n• Due", ""), "• Uno\n• Due");
    }
    #[test]
    fn salvataggio_unisce_e_restituisce_il_testo_finale() {
        let mut c = connection();
        assert_eq!(save(&mut c, "2026-10-12", "Spesa", "").unwrap(), "Spesa");
        // Il connettore MCP aggiunge una riga mentre l'utente sta scrivendo.
        c.execute(
            "UPDATE weekly_notes SET content=content||char(10)||'• Commercialista'",
            [],
        )
        .unwrap();
        let saved = save(&mut c, "2026-10-12", "Spesa e farmacia", "Spesa").unwrap();
        assert_eq!(saved, "Spesa e farmacia\n• Commercialista");
        assert_eq!(stored(&c), saved);
    }
}
