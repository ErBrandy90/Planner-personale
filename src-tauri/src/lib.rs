use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{Manager, State};

pub(crate) struct Database(pub(crate) Mutex<Connection>);
mod importing;
mod notes;

#[derive(Serialize)]
struct Entry {
    id: i64,
    week_start: String,
    day_date: String,
    content: String,
    position: i64,
    detected_time: Option<String>,
    scheduled_time: Option<String>,
    kind: String,
    completed: bool,
}
#[derive(Serialize)]
struct WeekData {
    entries: Vec<Entry>,
    notes: String,
}
#[derive(Serialize)]
struct WeekNote {
    week_start: String,
    content: String,
}
#[derive(Serialize)]
struct ExportFile {
    format: &'static str,
    schema_version: u32,
    entries: Vec<Entry>,
    weekly_notes: Vec<WeekNote>,
}

fn entry_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: r.get(0)?,
        week_start: r.get(1)?,
        day_date: r.get(2)?,
        content: r.get(3)?,
        position: r.get(4)?,
        detected_time: r.get(5)?,
        kind: r.get(6)?,
        completed: r.get::<_, i64>(7)? != 0,
        scheduled_time: r.get(8)?,
    })
}
fn get_entry(c: &Connection, id: i64) -> Result<Entry, String> {
    c.query_row("SELECT id,week_start,day_date,content,position,detected_time,kind,completed,scheduled_time FROM entries WHERE id=?1",[id],entry_from_row).map_err(|e|e.to_string())
}
fn lock<'a>(db: &'a State<'_, Database>) -> Result<std::sync::MutexGuard<'a, Connection>, String> {
    db.0.lock().map_err(|_| "Database non disponibile".into())
}

#[tauri::command]
fn load_week(db: State<Database>, week_start: String) -> Result<WeekData, String> {
    let c = lock(&db)?;
    let mut st=c.prepare("SELECT id,week_start,day_date,content,position,detected_time,kind,completed,scheduled_time FROM entries WHERE week_start=?1 ORDER BY day_date,position,id").map_err(|e|e.to_string())?;
    let entries = st
        .query_map([&week_start], entry_from_row)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    let notes = c
        .query_row(
            "SELECT content FROM weekly_notes WHERE week_start=?1",
            [week_start],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    Ok(WeekData { entries, notes })
}

#[tauri::command]
fn add_entry(
    db: State<Database>,
    week_start: String,
    day_date: String,
    content: String,
    position: i64,
    kind: String,
    detected_time: Option<String>,
) -> Result<Entry, String> {
    let c = lock(&db)?;
    c.execute("INSERT INTO entries(week_start,day_date,content,position,kind,detected_time) VALUES(?1,?2,?3,?4,?5,?6)",params![week_start,day_date,content,position,kind,detected_time]).map_err(|e|e.to_string())?;
    get_entry(&c, c.last_insert_rowid())
}

#[tauri::command]
fn update_entry(
    db: State<Database>,
    id: i64,
    content: String,
    kind: String,
    detected_time: Option<String>,
) -> Result<Entry, String> {
    let c = lock(&db)?;
    c.execute(
        "UPDATE entries SET content=?1,kind=?2,detected_time=?3 WHERE id=?4",
        params![content, kind, detected_time, id],
    )
    .map_err(|e| e.to_string())?;
    get_entry(&c, id)
}

#[tauri::command]
fn set_completed(db: State<Database>, id: i64, completed: bool) -> Result<(), String> {
    let c = lock(&db)?;
    c.execute(
        "UPDATE entries SET completed=?1 WHERE id=?2",
        params![completed as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_entry(db: State<Database>, id: i64) -> Result<(), String> {
    let c = lock(&db)?;
    c.execute("DELETE FROM entries WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn duplicate_entry(db: State<Database>, id: i64) -> Result<Entry, String> {
    let c = lock(&db)?;
    let e = get_entry(&c, id)?;
    let pos: i64 = c
        .query_row(
            "SELECT COALESCE(MAX(position),-1)+1 FROM entries WHERE day_date=?1",
            [&e.day_date],
            |r| r.get(0),
        )
        .map_err(|er| er.to_string())?;
    c.execute("INSERT INTO entries(week_start,day_date,content,position,detected_time,kind,completed,scheduled_time) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![e.week_start,e.day_date,e.content,pos,e.detected_time,e.kind,e.completed as i64,e.scheduled_time]).map_err(|er|er.to_string())?;
    get_entry(&c, c.last_insert_rowid())
}

#[tauri::command]
fn move_entry(db: State<Database>, id: i64, day_date: String) -> Result<Entry, String> {
    let c = lock(&db)?;
    let old = get_entry(&c, id)?;
    let pos = if old.scheduled_time.is_some() {
        old.position
    } else {
        c.query_row(
            "SELECT COALESCE(MAX(position),-1)+1 FROM entries WHERE day_date=?1",
            [&day_date],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?
    };
    c.execute(
        "UPDATE entries SET day_date=?1,position=?2 WHERE id=?3",
        params![day_date, pos, id],
    )
    .map_err(|e| e.to_string())?;
    get_entry(&c, id)
}

#[tauri::command]
fn set_entry_time(db: State<Database>, id: i64, time: Option<String>) -> Result<Entry, String> {
    let c = lock(&db)?;
    if let Some(ref value) = time {
        let parts: Vec<_> = value.split(':').collect();
        if parts.len() != 2 {
            return Err("Orario non valido".into());
        }
        let h: i64 = parts[0].parse().map_err(|_| "Orario non valido")?;
        let m: i64 = parts[1].parse().map_err(|_| "Orario non valido")?;
        if !(0..24).contains(&h) || !(0..60).contains(&m) {
            return Err("Orario non valido".into());
        }
        let position = (h - 7).max(0);
        c.execute(
            "UPDATE entries SET scheduled_time=?1,position=?2 WHERE id=?3",
            params![value, position, id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        c.execute("UPDATE entries SET scheduled_time=NULL WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
    }
    get_entry(&c, id)
}

#[tauri::command]
fn export_data(db: State<Database>, path: String) -> Result<(), String> {
    let c = lock(&db)?;
    let mut entries_stmt=c.prepare("SELECT id,week_start,day_date,content,position,detected_time,kind,completed,scheduled_time FROM entries ORDER BY week_start,day_date,position,id").map_err(|e|e.to_string())?;
    let entries = entries_stmt
        .query_map([], entry_from_row)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    let mut notes_stmt = c
        .prepare("SELECT week_start,content FROM weekly_notes ORDER BY week_start")
        .map_err(|e| e.to_string())?;
    let weekly_notes = notes_stmt
        .query_map([], |r| {
            Ok(WeekNote {
                week_start: r.get(0)?,
                content: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    let content = serde_json::to_vec_pretty(&ExportFile {
        format: "planner-settimanale",
        schema_version: 1,
        entries,
        weekly_notes,
    })
    .map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

fn database_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("planner.sqlite3"))
}

const SCHEMA: &str = "
    PRAGMA journal_mode=WAL;
    PRAGMA synchronous=FULL;
    CREATE TABLE IF NOT EXISTS entries (
        id INTEGER PRIMARY KEY,
        week_start TEXT NOT NULL,
        day_date TEXT NOT NULL,
        content TEXT NOT NULL,
        position INTEGER NOT NULL,
        detected_time TEXT,
        kind TEXT NOT NULL CHECK(kind IN ('text','task')),
        completed INTEGER NOT NULL DEFAULT 0,
        scheduled_time TEXT
    );
    CREATE INDEX IF NOT EXISTS entries_week_day ON entries(week_start,day_date,position);
    CREATE TABLE IF NOT EXISTS weekly_notes(week_start TEXT PRIMARY KEY,content TEXT NOT NULL);
";

/// Apre il database dell'app, crea le tabelle se mancano e aggiunge le colonne arrivate nelle versioni successive.
fn open_database(app: &tauri::AppHandle) -> Result<Connection, Box<dyn std::error::Error>> {
    let path = database_path(app).map_err(std::io::Error::other)?;
    Ok(prepare_database(Connection::open(path)?)?)
}

fn prepare_database(c: Connection) -> rusqlite::Result<Connection> {
    c.execute_batch(SCHEMA)?;
    let has_time: i64 = c.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('entries') WHERE name='scheduled_time'",
        [],
        |r| r.get(0),
    )?;
    if has_time == 0 {
        c.execute("ALTER TABLE entries ADD COLUMN scheduled_time TEXT", [])?;
    }
    Ok(c)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let c = open_database(app.handle())?;
            app.manage(Database(Mutex::new(c)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_week,
            add_entry,
            update_entry,
            set_completed,
            delete_entry,
            duplicate_entry,
            move_entry,
            set_entry_time,
            notes::save_notes,
            export_data,
            importing::inspect_import,
            importing::import_data,
            importing::backup_data
        ])
        .run(tauri::generate_context!())
        .expect("Errore avvio planner");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns(c: &Connection) -> Vec<String> {
        let mut st = c
            .prepare("SELECT name FROM pragma_table_info('entries')")
            .unwrap();
        st.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn database_nuovo_ha_tutte_le_tabelle() {
        let c = prepare_database(Connection::open_in_memory().unwrap()).unwrap();
        assert!(columns(&c).contains(&"scheduled_time".to_string()));
        let notes: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='weekly_notes'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(notes, 1);
        // aprirlo una seconda volta non deve dare errori
        prepare_database(c).unwrap();
    }

    #[test]
    fn database_vecchio_riceve_la_colonna_orario_senza_perdere_dati() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE entries (id INTEGER PRIMARY KEY,week_start TEXT NOT NULL,day_date TEXT NOT NULL,content TEXT NOT NULL,position INTEGER NOT NULL,detected_time TEXT,kind TEXT NOT NULL CHECK(kind IN ('text','task')),completed INTEGER NOT NULL DEFAULT 0);
             INSERT INTO entries VALUES (1,'2026-10-12','2026-10-13','[] Vecchia voce',0,NULL,'task',1);",
        )
        .unwrap();
        let c = prepare_database(c).unwrap();
        assert!(columns(&c).contains(&"scheduled_time".to_string()));
        let content: String = c
            .query_row("SELECT content FROM entries WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(content, "[] Vecchia voce");
    }
}
