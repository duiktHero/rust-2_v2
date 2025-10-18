use std::{
    env,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite;
use serde::{Deserialize, Serialize};

fn load_env() -> Result<()> {
    if let Ok(path) = env::var("SNIPPETS_ENV_FILE") {
        if let Err(e) = dotenvy::from_filename(&path) {
            eprintln!("WARN: failed to load SNIPPETS_ENV_FILE='{path}': {e}. Falling back...");
        } else {
            return Ok(());
        }
    }

    if dotenvy::dotenv().is_ok() {
        return Ok(());
    }

    let _ = dotenvy::from_filename("config/.env");
    Ok(())
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
        let file = File::open(&self.path)
            .with_context(|| format!("opening JSON storage '{}'", self.path.display()))?;
        let reader = BufReader::new(file);
        let v: Vec<Snippet> = serde_json::from_reader(reader)
            .with_context(|| format!("parsing JSON in '{}'", self.path.display()))?;
        Ok(v)
    }

    fn write_all(&self, all: &[Snippet]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating parent dir '{}'", parent.display()))?;
        }
        let file = File::create(&self.path)
            .with_context(|| format!("creating JSON storage '{}'", self.path.display()))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, all)
            .with_context(|| format!("writing JSON to '{}'", self.path.display()))?;
        Ok(())
    }
}
impl SnippetStorage for JsonStorage {
    fn list(&self) -> Result<Vec<Snippet>> {
        self.read_all().context("listing snippets from JSON")
    }

    fn insert(&self, snippet: &Snippet) -> Result<()> {
        let mut all = self.read_all().context("reading JSON before insert")?;
        all.push(snippet.clone());
        self.write_all(&all).context("persisting JSON after insert")
    }
}

struct SqliteStorage {
    conn: rusqlite::Connection,
}
impl SqliteStorage {
    fn new(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating sqlite parent dir '{}'", parent.display()))?;
        }
        let conn = rusqlite::Connection::open(path.as_ref())
            .with_context(|| format!("opening sqlite DB '{}'", path.as_ref().display()))?;

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
        )
        .context("initializing sqlite schema")?;

        Ok(Self { conn })
    }
}
impl SnippetStorage for SqliteStorage {
    fn list(&self) -> Result<Vec<Snippet>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, lang, code, created_at FROM snippets ORDER BY id ASC")
            .context("preparing SELECT for snippets")?;

        let rows = stmt
            .query_map([], |row| {
                let created_at_str: String = row.get(3)?;
                let created_at = created_at_str
                    .parse::<DateTime<Utc>>()
                    .map_err(|e| {
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
            })
            .context("executing SELECT for snippets")?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r.context("mapping sqlite row to Snippet")?);
        }
        Ok(out)
    }

    fn insert(&self, snippet: &Snippet) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO snippets (name, lang, code, created_at) VALUES (?1, ?2, ?3, ?4)",
                (
                    &snippet.name,
                    &snippet.lang,
                    &snippet.code,
                    &snippet.created_at.to_rfc3339(),
                ),
            )
            .with_context(|| format!("inserting snippet '{}'", snippet.name))?;
        Ok(())
    }
}

enum Provider {
    Json(PathBuf),
    Sqlite(PathBuf),
}

fn parse_storage_from_env() -> Result<Provider> {
    let raw = env::var("SNIPPETS_APP_STORAGE").unwrap_or_else(|_| "JSON:snippets.json".into());
    let (kind, path) = raw
        .split_once(':')
        .map(|(k, p)| (k.trim(), p.trim()))
        .unwrap_or(("JSON", "snippets.json"));

    if path.is_empty() {
        return Err(anyhow!("SNIPPETS_APP_STORAGE path part is empty (raw='{raw}')"));
    }

    let pb = PathBuf::from(path);
    let provider = match kind.to_uppercase().as_str() {
        "SQLITE" | "SQLITE3" => Provider::Sqlite(pb),
        "JSON" => Provider::Json(pb),
        other => {
            return Err(anyhow!(
                "unknown storage kind '{other}' in SNIPPETS_APP_STORAGE (raw='{raw}'). Supported: JSON, SQLITE"
            ))
        }
    };
    Ok(provider)
}

fn main() -> Result<()> {
    load_env().context("loading environment (.env)")?;
    let provider = parse_storage_from_env().context("parsing SNIPPETS_APP_STORAGE")?;

    let storage: Box<dyn SnippetStorage> = match &provider {
        Provider::Json(p) => {
            println!("Storage = JSON → {}", p.display());
            Box::new(JsonStorage::new(p))
        }
        Provider::Sqlite(p) => {
            println!("Storage = SQLITE → {}", p.display());
            Box::new(SqliteStorage::new(p).context("initializing sqlite storage")?)
        }
    };

    let mut all = storage.list().context("initial list() at startup")?;
    println!("Loaded {} snippet(s).", all.len());

    if all.is_empty() {
        let demo = Snippet {
            name: "hello".into(),
            lang: "rust".into(),
            code: r#"fn main() { println!("hi"); }"#.into(),
            created_at: Utc::now(),
        };
        storage
            .insert(&demo)
            .with_context(|| format!("inserting demo snippet '{}'", demo.name))?;
        all = storage.list().context("re-list() after inserting demo")?;
    }

    println!("Now you have {} snippet(s) in storage.", all.len());
    Ok(())
}
