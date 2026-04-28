use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub struct FocusEvent {
    pub id: i64,
    pub ts: DateTime<Utc>,
    pub app_name: String,
    pub bundle_id: String,
    pub window_title: String,
    pub previous_app: String,
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open() -> Result<Self> {
        let path = data_path();
        std::fs::create_dir_all(path.parent().unwrap()).ok();
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS focus_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts TEXT NOT NULL,
                app_name TEXT NOT NULL,
                bundle_id TEXT NOT NULL,
                window_title TEXT NOT NULL,
                previous_app TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ts ON focus_events(ts);
            "#,
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn insert(&self, ev: &FocusEvent) -> Result<i64> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO focus_events (ts, app_name, bundle_id, window_title, previous_app) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ev.ts.to_rfc3339(), ev.app_name, ev.bundle_id, ev.window_title, ev.previous_app],
        )?;
        Ok(c.last_insert_rowid())
    }

    pub fn load_all(&self) -> Result<Vec<FocusEvent>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT id, ts, app_name, bundle_id, window_title, previous_app FROM focus_events ORDER BY id DESC")?;
        let rows = stmt.query_map([], |row| {
            let ts_str: String = row.get(1)?;
            let ts = DateTime::parse_from_rfc3339(&ts_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(FocusEvent {
                id: row.get(0)?,
                ts,
                app_name: row.get(2)?,
                bundle_id: row.get(3)?,
                window_title: row.get(4)?,
                previous_app: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn clear(&self) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute("DELETE FROM focus_events", [])?;
        Ok(())
    }
}

pub fn data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("FocusTrace")
}

pub fn data_path() -> PathBuf {
    data_dir().join("focus.sqlite")
}
