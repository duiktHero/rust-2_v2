use std::{
    env,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, trace, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

fn load_env() -> Result<()> {
    if let Ok(path) = env::var("SNIPPETS_ENV_FILE") {
        if let Err(e) = dotenvy::from_filename(&path) {
            eprintln!("WARN: failed to load SNIPPETS_ENV_FILE='{}': {e}. Falling back...", path);
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


fn init_tracing() -> Result<Option<WorkerGuard>> {
    let level = env::var("SNIPPETS_APP_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));

    if let Ok(path) = env::var("SNIPPETS_APP_LOG_PATH") {
        let log_path = PathBuf::from(path);
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("opening log file '{}'", log_path.display()))?;
        let (nb, guard) = tracing_appender::non_blocking(file);
        fmt()
            .with_writer(nb)
            .with_ansi(false)
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_level(true)
            .init();
        Ok(Some(guard))
    } else {
        fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_level(true)
            .init();
        Ok(None)
    }
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
    fn get(&self, name: &str) -> Result<Option<Snippet>>;
    fn insert(&self, snippet: &Snippet) -> Result<()>;
    fn remove(&self, name: &str) -> Result<bool>;
}

struct JsonStorage {
    path: PathBuf,
}

impl JsonStorage {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn read_all(&self) -> Result<Vec<Snippet>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let file = File::open(&self.path)
            .with_context(|| format!("opening JSON storage '{}'", self.path.display()))?;
        let mut buf = String::new();
        io::BufReader::new(file)
            .read_to_string(&mut buf)
            .with_context(|| format!("reading '{}'", self.path.display()))?;
        if buf.trim().is_empty() {
            return Ok(vec![]);
        }
        let v: Vec<Snippet> = serde_json::from_str(&buf)
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
        serde_json::to_writer_pretty(io::BufWriter::new(file), all)?;
        Ok(())
    }
}

impl SnippetStorage for JsonStorage {
    fn list(&self) -> Result<Vec<Snippet>> {
        self.read_all()
    }

    fn get(&self, name: &str) -> Result<Option<Snippet>> {
        let all = self.read_all()?;
        Ok(all.into_iter().find(|s| s.name == name))
    }

    fn insert(&self, snippet: &Snippet) -> Result<()> {
        let mut all = self.read_all()?;
        if let Some(pos) = all.iter().position(|s| s.name == snippet.name) {
            all[pos] = snippet.clone();
        } else {
            all.push(snippet.clone());
        }
        self.write_all(&all)
    }

    fn remove(&self, name: &str) -> Result<bool> {
        let mut all = self.read_all()?;
        let len_before = all.len();
        all.retain(|s| s.name != name);
        self.write_all(&all)?;
        Ok(all.len() != len_before)
    }
}

/* SQLite storage */
struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    fn new(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
        CREATE TABLE IF NOT EXISTS snippets (
          name TEXT PRIMARY KEY,
          lang TEXT NOT NULL,
          code TEXT NOT NULL,
          created_at TEXT NOT NULL
        );
        "#,
        )?;
        Ok(Self { conn })
    }
}

impl SnippetStorage for SqliteStorage {
    fn list(&self) -> Result<Vec<Snippet>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, lang, code, created_at FROM snippets ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let created_at_str: String = row.get(3)?;
            let dt = created_at_str
                .parse::<DateTime<Utc>>()
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
            Ok(Snippet {
                name: row.get(0)?,
                lang: row.get(1)?,
                code: row.get(2)?,
                created_at: dt,
            })
        })?;
        let mut v = Vec::new();
        for it in rows {
            v.push(it?);
        }
        Ok(v)
    }

    fn get(&self, name: &str) -> Result<Option<Snippet>> {
        let row = self.conn.query_row(
            "SELECT name, lang, code, created_at FROM snippets WHERE name = ?1",
            params![name],
            |row| {
                let created_at_str: String = row.get(3)?;
                let dt = created_at_str
                    .parse::<DateTime<Utc>>()
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
                Ok(Snippet {
                    name: row.get(0)?,
                    lang: row.get(1)?,
                    code: row.get(2)?,
                    created_at: dt,
                })
            },
        ).optional()?;
        Ok(row)
    }

    fn insert(&self, snippet: &Snippet) -> Result<()> {
        self.conn.execute(
            "INSERT INTO snippets(name, lang, code, created_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(name) DO UPDATE SET lang = excluded.lang, code = excluded.code, created_at = excluded.created_at",
            params![snippet.name, snippet.lang, snippet.code, snippet.created_at.to_rfc3339()],
        )?;
        Ok(())
    }

    fn remove(&self, name: &str) -> Result<bool> {
        let n = self.conn.execute("DELETE FROM snippets WHERE name = ?1", params![name])?;
        Ok(n > 0)
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Backend {
    Json,
    Sqlite,
}

#[derive(Parser, Debug)]
#[command(
    name = "snippets-app",
    version,
    about = "Stores code snippets into JSON or SQLite and can download from URL",
    disable_help_subcommand = true
)]
struct Cli {

    #[arg(short = 'n', long = "name", value_name = "NAME")]
    name: Option<String>,


    #[arg(short = 'l', long = "lang", default_value = "text")]
    lang: String,

    #[arg(short = 'b', long = "backend", value_enum, default_value_t = Backend::Json)]
    backend: Backend,

    #[arg(long = "json-path", value_name = "PATH", default_value = "snippets.json")]
    json_path: PathBuf,

    #[arg(long = "sqlite-path", value_name = "PATH", default_value = "snippets.sqlite3")]
    sqlite_path: PathBuf,

    #[arg(long = "download", value_name = "URL")]
    download_url: Option<String>,

    #[arg(long = "list", action = ArgAction::SetTrue)]
    list: bool,

    #[arg(long = "remove", value_name = "NAME")]
    remove: Option<String>,
}

fn make_storage(cli: &Cli) -> Result<Box<dyn SnippetStorage>> {
    match cli.backend {
        Backend::Json => Ok(Box::new(JsonStorage::new(&cli.json_path))),
        Backend::Sqlite => Ok(Box::new(SqliteStorage::new(&cli.sqlite_path)?)),
    }
}

fn read_from_stdin() -> Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).context("reading from STDIN")?;
    if buf.is_empty() {
        Err(anyhow!("no data in STDIN; pass --download <URL> or pipe content"))
    } else {
        Ok(buf)
    }
}

fn download(url: &str) -> Result<String> {
    info!(%url, "Downloading snippet");
    let resp = reqwest::blocking::get(url).with_context(|| format!("GET {}", url))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow!("HTTP {}", status));
    }
    let body = resp.text().context("reading response body")?;
    Ok(body)
}

fn main() -> Result<()> {
    load_env().ok();
    let _guard = init_tracing()?;

    let cli = Cli::parse();
    trace!("CLI parsed: {:?}", cli);


    let storage = make_storage(&cli)?;

    if cli.list {
        let items = storage.list()?;
        for s in &items {
            println!("{} [{}]  @{}", s.name, s.lang, s.created_at.to_rfc3339());
        }
        info!(count = items.len(), "Listed snippets");
        return Ok(());
    }

    if let Some(name) = cli.remove.as_deref() {
        let removed = storage.remove(name)?;
        if removed {
            println!("Removed '{name}'");
            info!(%name, "Removed snippet");
        } else {
            println!("No snippet named '{name}'");
            warn!(%name, "Remove requested but snippet did not exist");
        }
        return Ok(());
    }

    let name = cli
        .name
        .as_deref()
        .ok_or_else(|| anyhow!("--name <NAME> is required unless --list or --remove is used"))?;


    let code = if let Some(url) = cli.download_url.as_deref() {
        download(url)?
    } else {
        read_from_stdin()? 
    };

    let snippet = Snippet {
        name: name.to_string(),
        lang: cli.lang,
        code,
        created_at: Utc::now(),
    };

    storage.insert(&snippet)?;
    println!("Saved snippet '{}'", snippet.name);
    info!(name = %snippet.name, bytes = snippet.code.len(), backend = ?cli.backend, "Snippet saved");
    Ok(())
}
