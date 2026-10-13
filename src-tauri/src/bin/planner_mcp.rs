use chrono::{Datelike, Duration, Local, NaiveDate};
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt, transport::stdio,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone)]
struct PlannerMcp {
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ListTasks {
    /// Prima data da includere, nel formato YYYY-MM-DD. Se omessa usa oggi.
    date_from: Option<String>,
    /// Ultima data da includere, nel formato YYYY-MM-DD. Se omessa usa 14 giorni dopo date_from.
    date_to: Option<String>,
    /// Se true include anche le attività completate.
    include_completed: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CreateTask {
    /// Testo dell'attività, senza la sintassi [].
    title: String,
    /// Data nel formato YYYY-MM-DD.
    date: String,
    /// Orario facoltativo nel formato HH:MM, 24 ore.
    time: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CompleteTask {
    /// ID numerico restituito da list_tasks o create_task.
    id: i64,
    /// true per completata, false per riaprirla.
    completed: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AddWeeklyNotes {
    /// Una data qualsiasi della settimana destinataria, in formato YYYY-MM-DD. Se omessa usa oggi.
    week_of: Option<String>,
    /// Impegni non ancora assegnati a un giorno/orario. Ogni elemento viene aggiunto come punto elenco.
    items: Vec<String>,
}

#[derive(Debug, Serialize)]
struct TaskView {
    id: i64,
    date: String,
    time: Option<String>,
    title: String,
    completed: bool,
}

fn db_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("PLANNER_DB_PATH") {
        return Ok(PathBuf::from(path));
    }
    dirs::data_dir()
        .map(|p| p.join("it.planner.settimanale").join("planner.sqlite3"))
        .ok_or_else(|| "Impossibile trovare la cartella dati di Windows".to_string())
}

fn open_db() -> Result<Connection, String> {
    let path = db_path()?;
    if !path.exists() {
        return Err(format!("Database del planner non trovato in {}. Avvia il Planner Settimanale almeno una volta.", path.display()));
    }
    let connection = Connection::open(path).map_err(|e| e.to_string())?;
    connection.busy_timeout(std::time::Duration::from_secs(5)).map_err(|e| e.to_string())?;
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;").map_err(|e| e.to_string())?;
    Ok(connection)
}

fn parse_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| format!("Data non valida: {value}. Usa YYYY-MM-DD."))
}

fn normalize_time(value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = value else { return Ok(None) };
    let mut parts = value.split(':');
    let hour: u32 = parts.next().and_then(|v| v.parse().ok()).ok_or_else(|| "Orario non valido. Usa HH:MM.".to_string())?;
    let minute: u32 = parts.next().and_then(|v| v.parse().ok()).ok_or_else(|| "Orario non valido. Usa HH:MM.".to_string())?;
    if parts.next().is_some() || hour > 23 || minute > 59 { return Err("Orario non valido. Usa HH:MM.".into()); }
    Ok(Some(format!("{hour:02}:{minute:02}")))
}

fn monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

/// Risultato di uno strumento MCP: JSON leggibile se va bene. Un `Err` arriva al client come
/// risultato con `isError: true`, così l'assistente capisce che l'operazione non è riuscita.
fn result<T: Serialize>(value: Result<T, String>) -> Result<String, String> {
    let value = value.map_err(|error| format!("Errore: {error}"))?;
    serde_json::to_string_pretty(&value).map_err(|e| format!("Errore: {e}"))
}

fn clean_title(content: String) -> String {
    content.strip_prefix("[] ").or_else(|| content.strip_prefix("[ ] ")).unwrap_or(&content).to_string()
}

#[tool_router]
impl PlannerMcp {
    fn new() -> Self {
        Self { tool_router: Self::tool_router() }
    }

    #[tool(description = "Legge le attività del Planner Settimanale in un intervallo di date. Usalo prima di aggiungere attività per evitare duplicati e per ottenere gli ID necessari a completarle.")]
    fn list_tasks(&self, Parameters(input): Parameters<ListTasks>) -> Result<String, String> {
        result((|| {
            let start = input.date_from.as_deref().map(parse_date).transpose()?.unwrap_or_else(|| Local::now().date_naive());
            let end = input.date_to.as_deref().map(parse_date).transpose()?.unwrap_or(start + Duration::days(14));
            if end < start { return Err("date_to non può precedere date_from".into()); }
            let connection = open_db()?;
            let mut statement = connection.prepare("SELECT id,day_date,scheduled_time,content,completed FROM entries WHERE kind='task' AND day_date BETWEEN ?1 AND ?2 AND (?3=1 OR completed=0) ORDER BY day_date,position,id").map_err(|e| e.to_string())?;
            let tasks = statement.query_map(params![start.to_string(), end.to_string(), input.include_completed.unwrap_or(false) as i64], |row| {
                Ok(TaskView { id: row.get(0)?, date: row.get(1)?, time: row.get(2)?, title: clean_title(row.get(3)?), completed: row.get::<_, i64>(4)? != 0 })
            }).map_err(|e| e.to_string())?.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())?;
            Ok(tasks)
        })())
    }

    #[tool(description = "Crea una nuova attività con checkbox nel Planner Settimanale. Chiedi conferma all'utente prima di scrivere se data, testo o orario sono ambigui.")]
    fn create_task(&self, Parameters(input): Parameters<CreateTask>) -> Result<String, String> {
        result((|| {
            let title = input.title.trim();
            if title.is_empty() { return Err("Il testo dell'attività è vuoto".into()); }
            let date = parse_date(&input.date)?;
            let time = normalize_time(input.time)?;
            let connection = open_db()?;
            let position = if let Some(ref time) = time {
                time[..2].parse::<i64>().unwrap_or(7).saturating_sub(7)
            } else {
                connection.query_row("SELECT COALESCE(MAX(position),-1)+1 FROM entries WHERE day_date=?1", [date.to_string()], |row| row.get(0)).map_err(|e| e.to_string())?
            };
            connection.execute("INSERT INTO entries(week_start,day_date,content,position,detected_time,scheduled_time,kind,completed) VALUES(?1,?2,?3,?4,?5,?6,'task',0)", params![monday(date).to_string(), date.to_string(), format!("[] {title}"), position, time, time]).map_err(|e| e.to_string())?;
            Ok(TaskView { id: connection.last_insert_rowid(), date: date.to_string(), time, title: title.to_string(), completed: false })
        })())
    }

    #[tool(description = "Segna come completata oppure riapre un'attività del Planner Settimanale usando il suo ID.")]
    fn set_task_completed(&self, Parameters(input): Parameters<CompleteTask>) -> Result<String, String> {
        result((|| {
            let connection = open_db()?;
            let changed = connection.execute("UPDATE entries SET completed=?1 WHERE id=?2 AND kind='task'", params![input.completed as i64, input.id]).map_err(|e| e.to_string())?;
            if changed == 0 { return Err(format!("Attività {} non trovata", input.id)); }
            Ok(serde_json::json!({"id": input.id, "completed": input.completed}))
        })())
    }

    #[tool(description = "Aggiunge alle note di una settimana gli impegni non ancora collocati in agenda. Usa questa operazione quando manca una data precisa; non creare una task giornaliera inventando una data.")]
    fn add_weekly_notes(&self, Parameters(input): Parameters<AddWeeklyNotes>) -> Result<String, String> {
        result((|| {
            let date = input.week_of.as_deref().map(parse_date).transpose()?.unwrap_or_else(|| Local::now().date_naive());
            let week_start = monday(date).to_string();
            append_weekly_notes(&mut open_db()?, &week_start, input.items)
        })())
    }
}

/// Aggiunge le righe in fondo alle note della settimana, saltando quelle già presenti.
/// Se le note esistenti non si leggono, si ferma con un errore invece di riscriverle da zero.
fn append_weekly_notes(connection: &mut Connection, week_start: &str, items: Vec<String>) -> Result<serde_json::Value, String> {
    // Transazione esclusiva: lettura e scrittura avvengono senza che l'app salvi le note nel mezzo.
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    let existing: String = transaction.query_row("SELECT content FROM weekly_notes WHERE week_start=?1", [week_start], |row| row.get(0))
        .optional().map_err(|e| format!("Note della settimana {week_start} non leggibili, nessuna modifica fatta: {e}"))?.unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(str::to_string).collect();
    let mut added = Vec::new();
    let mut skipped = Vec::new();
    for item in items {
        let clean = item.trim().trim_start_matches(['•','-']).trim();
        if clean.is_empty() { continue; }
        let line = format!("• {clean}");
        if lines.iter().any(|present| present.trim().eq_ignore_ascii_case(&line)) { skipped.push(clean.to_string()); }
        else { lines.push(line); added.push(clean.to_string()); }
    }
    let content = lines.join("\n").trim().to_string();
    transaction.execute("INSERT INTO weekly_notes(week_start,content) VALUES(?1,?2) ON CONFLICT(week_start) DO UPDATE SET content=excluded.content", params![week_start,content]).map_err(|e| e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"week_start": week_start, "added": added, "already_present": skipped}))
}

#[tool_handler]
impl ServerHandler for PlannerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Gestisce il Planner Settimanale locale. Prima di creare attività, controlla quelle esistenti per evitare duplicati. Se un impegno non ha ancora una data precisa, aggiungilo alle note della settimana invece di inventare una data. Chiedi conferma quando la settimana è ambigua.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = PlannerMcp::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn connection() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE weekly_notes(week_start TEXT PRIMARY KEY,content);").unwrap(); c
    }
    fn items(values: &[&str]) -> Vec<String> { values.iter().map(|v| v.to_string()).collect() }

    #[test]
    fn aggiunge_in_fondo_e_salta_i_duplicati() {
        let mut c = connection();
        c.execute("INSERT INTO weekly_notes VALUES('2026-10-12','Spesa'||char(10)||'• Dentista')", []).unwrap();
        let report = append_weekly_notes(&mut c, "2026-10-12", items(&["dentista", "- Bollo auto", "  "])).unwrap();
        assert_eq!(report["added"], serde_json::json!(["Bollo auto"]));
        assert_eq!(report["already_present"], serde_json::json!(["dentista"]));
        let saved: String = c.query_row("SELECT content FROM weekly_notes", [], |r| r.get(0)).unwrap();
        assert_eq!(saved, "Spesa\n• Dentista\n• Bollo auto");
    }
    #[test]
    fn settimana_senza_note_crea_la_riga() {
        let mut c = connection();
        append_weekly_notes(&mut c, "2026-10-19", items(&["Commercialista"])).unwrap();
        let saved: String = c.query_row("SELECT content FROM weekly_notes WHERE week_start='2026-10-19'", [], |r| r.get(0)).unwrap();
        assert_eq!(saved, "• Commercialista");
    }
    #[test]
    fn gli_errori_arrivano_al_client_come_errori() {
        use rmcp::handler::server::tool::IntoCallToolResult;
        let failed = result::<()>(Err("Attività 99 non trovata".into())).into_call_tool_result().unwrap();
        assert_eq!(failed.is_error, Some(true));
        let ok = result(Ok(serde_json::json!({"id": 1}))).into_call_tool_result().unwrap();
        assert_eq!(ok.is_error, Some(false));
    }
    #[test]
    fn note_non_leggibili_non_vengono_cancellate() {
        // Prima della correzione un errore di lettura valeva "note vuote" e la riga veniva riscritta da zero.
        let mut c = connection();
        c.execute("INSERT INTO weekly_notes VALUES('2026-10-12', X'00FF')", []).unwrap();
        let error = append_weekly_notes(&mut c, "2026-10-12", items(&["Nuova"])).unwrap_err();
        assert!(error.contains("non leggibili"), "{error}");
        let still: Vec<u8> = c.query_row("SELECT content FROM weekly_notes", [], |r| r.get(0)).unwrap();
        assert_eq!(still, vec![0x00, 0xFF]);
    }
}
