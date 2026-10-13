use super::Database;
use chrono::{Datelike, NaiveDate, Weekday};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, State};

const MAX_BYTES: usize = 20 * 1024 * 1024;
#[derive(Deserialize)]
struct ImportedEntry {
    id: i64,
    week_start: String,
    day_date: String,
    content: String,
    position: i64,
    detected_time: Option<String>,
    #[serde(default)]
    scheduled_time: Option<String>,
    kind: String,
    completed: bool,
}
#[derive(Deserialize)]
struct Note {
    week_start: String,
    content: String,
}
#[derive(Deserialize)]
struct ImportFile {
    format: String,
    schema_version: u32,
    entries: Vec<ImportedEntry>,
    weekly_notes: Vec<Note>,
}
#[derive(Serialize)]
pub(crate) struct Preview {
    content: String,
    entries: usize,
    notes: usize,
}
#[derive(Serialize, Default)]
pub(crate) struct Report {
    added: usize,
    skipped: usize,
    notes_updated: usize,
    backup_path: String,
}

fn date(value: &str) -> Result<NaiveDate, String> {
    let parsed = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| format!("Data non valida: {value}"))?;
    if parsed.format("%Y-%m-%d").to_string() != value {
        return Err("Usare date YYYY-MM-DD".into());
    }
    Ok(parsed)
}
fn week(value: &str) -> Result<NaiveDate, String> {
    let d = date(value)?;
    if d.weekday() != Weekday::Mon {
        return Err(format!("La settimana deve iniziare di lunedì: {value}"));
    }
    Ok(d)
}
fn valid_time(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|s| {
        let bytes = s.as_bytes();
        bytes.len() == 5
            && bytes[2] == b':'
            && bytes
                .iter()
                .enumerate()
                .all(|(i, b)| i == 2 || b.is_ascii_digit())
            && s[..2].parse::<u8>().is_ok_and(|h| h < 24)
            && s[3..].parse::<u8>().is_ok_and(|m| m < 60)
    })
}
fn parse(content: &str) -> Result<ImportFile, String> {
    if content.len() > MAX_BYTES {
        return Err("Il file supera il limite di 20 MB".into());
    }
    let data: ImportFile = serde_json::from_str(content.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("JSON del planner non valido: {e}"))?;
    if data.format != "planner-settimanale" || data.schema_version != 1 {
        return Err("Formato o versione del file non supportati".into());
    }
    let mut ids = HashSet::new();
    for e in &data.entries {
        let start = week(&e.week_start)?;
        let delta = date(&e.day_date)?.signed_duration_since(start).num_days();
        if !(0..7).contains(&delta)
            || e.id <= 0
            || !ids.insert(e.id)
            || !(0..=10000).contains(&e.position)
            || e.content.trim().is_empty()
            || !matches!(e.kind.as_str(), "text" | "task")
            || !valid_time(&e.detected_time)
            || !valid_time(&e.scheduled_time)
        {
            return Err(format!("Voce non valida nel file (ID {})", e.id));
        }
    }
    let mut weeks = HashSet::new();
    for n in &data.weekly_notes {
        week(&n.week_start)?;
        if !weeks.insert(&n.week_start) {
            return Err("Note duplicate per la stessa settimana nel file".into());
        }
    }
    Ok(data)
}

pub(crate) fn backup(c: &Connection, directory: &Path) -> Result<String, String> {
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let path = directory.join(format!("planner-{stamp}.sqlite3"));
    let path_text = path.to_string_lossy().into_owned();
    // VACUUM INTO captures a consistent snapshot, including committed WAL data.
    c.execute("VACUUM INTO ?1", [&path_text])
        .map_err(|e| format!("Backup non riuscito: {e}"))?;
    Ok(path_text)
}

fn merge(c: &mut Connection, data: ImportFile) -> Result<Report, String> {
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let mut report = Report::default();
    for e in data.entries {
        let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM entries WHERE week_start=?1 AND day_date=?2 AND content=?3 AND position=?4 AND detected_time IS ?5 AND scheduled_time IS ?6 AND kind=?7 AND completed=?8)",
            params![e.week_start,e.day_date,e.content,e.position,e.detected_time,e.scheduled_time,e.kind,e.completed as i64], |r|r.get(0)).map_err(|e|e.to_string())?;
        if exists {
            report.skipped += 1;
            continue;
        }
        // Exported IDs are local to the source database: assign new IDs here.
        tx.execute("INSERT INTO entries(week_start,day_date,content,position,detected_time,scheduled_time,kind,completed) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![e.week_start,e.day_date,e.content,e.position,e.detected_time,e.scheduled_time,e.kind,e.completed as i64]).map_err(|e|e.to_string())?;
        report.added += 1;
    }
    for n in data.weekly_notes {
        if n.content.trim().is_empty() {
            continue;
        }
        let old: String = tx
            .query_row(
                "SELECT COALESCE((SELECT content FROM weekly_notes WHERE week_start=?1),'')",
                [&n.week_start],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if old.contains(&n.content) {
            continue;
        }
        let value = if old.is_empty() {
            n.content
        } else {
            format!("{old}\n\n{}", n.content)
        };
        tx.execute("INSERT INTO weekly_notes(week_start,content) VALUES(?1,?2) ON CONFLICT(week_start) DO UPDATE SET content=excluded.content", params![n.week_start,value]).map_err(|e|e.to_string())?;
        report.notes_updated += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(report)
}

#[tauri::command]
pub(crate) fn inspect_import(path: String) -> Result<Preview, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    // Bound the actual read, even if the file changes after metadata is read.
    use std::io::Read;
    let mut content = String::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_string(&mut content)
        .map_err(|e| e.to_string())?;
    let data = parse(&content)?;
    Ok(Preview {
        entries: data.entries.len(),
        notes: data.weekly_notes.len(),
        content,
    })
}
#[tauri::command]
pub(crate) fn import_data(
    app: tauri::AppHandle,
    db: State<Database>,
    content: String,
) -> Result<Report, String> {
    // Revalidate the exact previewed content; do not reopen a possibly modified file.
    let data = parse(&content)?;
    let mut c = db.0.lock().map_err(|_| "Database non disponibile")?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    let backup_path = backup(&c, &directory)?;
    let mut result = merge(&mut c, data)?;
    result.backup_path = backup_path;
    Ok(result)
}
#[tauri::command]
pub(crate) fn backup_data(app: tauri::AppHandle, db: State<Database>) -> Result<String, String> {
    let c = db.0.lock().map_err(|_| "Database non disponibile")?;
    backup(
        &c,
        &app.path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("backups"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn connection() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE entries(id INTEGER PRIMARY KEY,week_start TEXT,day_date TEXT,content TEXT,position INTEGER,detected_time TEXT,scheduled_time TEXT,kind TEXT,completed INTEGER); CREATE TABLE weekly_notes(week_start TEXT PRIMARY KEY,content TEXT);").unwrap();
        c
    }
    fn fixture() -> String {
        serde_json::json!({"format":"planner-settimanale","schema_version":1,"entries":[{"id":1,"week_start":"2026-10-12","day_date":"2026-10-13","content":"[] 09:30 CAF","position":2,"detected_time":"09:30","scheduled_time":"09:30","kind":"task","completed":true}],"weekly_notes":[{"week_start":"2026-10-12","content":"Note importate"}]}).to_string()
    }
    #[test]
    fn merge_is_repeatable_and_preserves_existing_data() {
        let mut c = connection();
        c.execute("INSERT INTO entries VALUES(1,'2026-10-12','2026-10-14','Esistente',0,NULL,NULL,'text',0)",[]).unwrap();
        c.execute(
            "INSERT INTO weekly_notes VALUES('2026-10-12','Note esistenti')",
            [],
        )
        .unwrap();
        let first = merge(&mut c, parse(&fixture()).unwrap()).unwrap();
        assert_eq!(first.added, 1);
        assert_eq!(first.notes_updated, 1);
        let second = merge(&mut c, parse(&fixture()).unwrap()).unwrap();
        assert_eq!(second.added, 0);
        assert_eq!(second.skipped, 1);
        assert_eq!(second.notes_updated, 0);
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            c.query_row("SELECT content FROM weekly_notes", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "Note esistenti\n\nNote importate"
        );
        assert_eq!(
            c.query_row("SELECT completed FROM entries WHERE id=2", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn malformed_exports_are_rejected() {
        let original: serde_json::Value = serde_json::from_str(&fixture()).unwrap();
        for (field, value) in [
            ("day_date", "2026-10-20"),
            ("week_start", "2026-10-13"),
            ("scheduled_time", "25:10"),
            ("detected_time", "é:00"),
            ("kind", "unknown"),
            ("content", ""),
        ] {
            let mut v = original.clone();
            v["entries"][0][field] = value.into();
            assert!(parse(&v.to_string()).is_err(), "{field}");
        }
        let mut v = original.clone();
        v["schema_version"] = 2.into();
        assert!(parse(&v.to_string()).is_err());
        let mut v = original.clone();
        v["entries"][0]["position"] = (-1).into();
        assert!(parse(&v.to_string()).is_err());
        let mut v = original.clone();
        v["entries"]
            .as_array_mut()
            .unwrap()
            .push(original["entries"][0].clone());
        assert!(parse(&v.to_string()).is_err());
        assert!(parse("{}").is_err());
    }
    #[test]
    fn older_exports_without_scheduled_time_are_supported() {
        let mut v: serde_json::Value = serde_json::from_str(&fixture()).unwrap();
        v["entries"][0]
            .as_object_mut()
            .unwrap()
            .remove("scheduled_time");
        assert!(parse(&v.to_string()).unwrap().entries[0]
            .scheduled_time
            .is_none());
    }
    #[test]
    fn failed_merge_rolls_back_all_changes() {
        let mut c = connection();
        c.execute_batch("CREATE TRIGGER fail_notes BEFORE INSERT ON weekly_notes BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(merge(&mut c, parse(&fixture()).unwrap()).is_err());
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn backup_can_be_opened_and_contains_original_data() {
        let mut c = connection();
        merge(&mut c, parse(&fixture()).unwrap()).unwrap();
        let directory = std::env::temp_dir().join(format!(
            "planner-import-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = backup(&c, &directory).unwrap();
        let saved = Connection::open(&path).unwrap();
        assert_eq!(
            saved
                .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(saved);
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
