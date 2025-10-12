use std::{
    env,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite;
use serde::{Deserialize, Serialize};

fn load_env() {
    if let Ok(path) = env::var("SNIPPETS_ENV_FILE") {
        match dotenvy::from_filename(&path) {
            Ok(_) => return,
            Err(e) => eprintln!("WARN: failed to load SNIPPETS_ENV_FILE='{path}': {e}. Falling back..."),
        }
    }
    if dotenvy::dotenv().is_ok() {
        return;
    }
    let _ = dotenvy::from_filename("config/.env");
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Snippet {
    name: String,
    lang: String,
    code: String,
    created_at: DateTime<Utc>,
}

trait SnippetStorage {
    fn list(&self) -> Result<Vec<Snippet>>;
    fn insert(&self, snippet: &Snippet) -> Result<()>;
}

/* JSON storage */
struct JsonStorage {
    path: PathBuf,
}
impl JsonStorage {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
    fn read_all(&self) -> Result<Vec<Snippet>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let v: Vec<Snippet> = serde_json::from_reader(reader)?;
        Ok(v)
    }
    fn write_all(&self, all: &[Snippet]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(&self.path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, all)?;
        Ok(())
    }
}
impl SnippetStorage for JsonStorage {
    fn list(&self) -> Result<Vec<Snippet>> { self.read_all() }
    fn insert(&self, snippet: &Snippet) -> Result<()> {
        let mut all = self.read_all()?;
        all.push(snippet.clone());
        self.write_all(&all)
    }
}

struct SqliteStorage {
    conn: rusqlite::Connection,
}
impl SqliteStorage {
    fn new(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS snippets (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT NOT NULL,
                lang        TEXT NOT NULL,
                code        TEXT NOT NULL,
                created_at  TEXT NOT NULL
            );
            "#,
        )?;
        Ok(Self { conn })
    }
}
impl SnippetStorage for SqliteStorage {
    fn list(&self) -> Result<Vec<Snippet>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, lang, code, created_at FROM snippets ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let created_at_str: String = row.get(3)?;
            let created_at = created_at_str.parse::<DateTime<Utc>>().map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(Snippet {
                name: row.get(0)?,
                lang: row.get(1)?,
                code: row.get(2)?,
                created_at,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
    fn insert(&self, snippet: &Snippet) -> Result<()> {
        self.conn.execute(
            "INSERT INTO snippets (name, lang, code, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                &snippet.name,
                &snippet.lang,
                &snippet.code,
                &snippet.created_at.to_rfc3339(),
            ),
        )?;
        Ok(())
    }
}

/* ENV → provider */
enum Provider {
    Json(PathBuf),
    Sqlite(PathBuf),
}
fn parse_storage_from_env() -> Provider {
    let raw = env::var("SNIPPETS_APP_STORAGE").unwrap_or_else(|_| "JSON:snippets.json".into());
    let (kind, path) = raw.split_once(':').map(|(k, p)| (k.trim(), p.trim()))
        .unwrap_or(("JSON", "snippets.json"));
    let pb = PathBuf::from(path);
    match kind.to_uppercase().as_str() {
        "SQLITE" | "SQLITE3" => Provider::Sqlite(pb),
        "JSON" => Provider::Json(pb),
        _ => Provider::Json(pb),
    }
}

fn main() -> Result<()> {
    load_env();
    let provider = parse_storage_from_env();

    let storage: Box<dyn SnippetStorage> = match &provider {
        Provider::Json(p) => {
            println!("Storage = JSON → {}", p.display());
            Box::new(JsonStorage::new(p))
        }
        Provider::Sqlite(p) => {
            println!("Storage = SQLITE → {}", p.display());
            Box::new(SqliteStorage::new(p)?)
        }
    };

    let mut all = storage.list()?;
    println!("Loaded {} snippet(s).", all.len());

    if all.is_empty() {
        let demo = Snippet {
            name: "hello".into(),
            lang: "rust".into(),
            code: r#"fn main() { println!("hi"); }"#.into(),
            created_at: Utc::now(),
        };
        storage.insert(&demo)?;
        all = storage.list()?;
    }

    println!("Now you have {} snippet(s) in storage.", all.len());
    Ok(())
}
