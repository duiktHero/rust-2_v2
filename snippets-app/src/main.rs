//! `snippets-app` — компактна CLI-утиліта для збереження та перегляду код-сніпетів у JSON.
//!
//! Можливості:
//! - додавання сніпету з stdin або завантаженням по `--download <URL>`;
//! - перелік та видалення сніпетів;
//! - логування з рівнем/шляхом через змінні середовища:
//!   - `SNIPPETS_APP_LOG_LEVEL` (наприклад, `info`, `debug`)
//!   - `SNIPPETS_APP_LOG_PATH` (якщо не задано — лог у stdout).
//!
//! Запуск прикладів:
//! ```bash
//! echo "println!(\"hi\");" | snippets-app add -n hello -l rust
//! snippets-app add -n logger --download "https://example.com/snippet.rs"
//! snippets-app list
//! snippets-app remove -n hello
//! ```

#![deny(
    missing_docs,
    broken_intra_doc_links,
    missing_crate_level_docs,
    unreachable_pub
)]
#![deny(
    clippy::missing_panics_doc,
    clippy::clone_on_ref_ptr,
    clippy::similar_names
)]

use std::{
    collections::BTreeMap,
    env,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

/// Конфігурація застосунку (рівень/шлях логів і шлях до JSON).
#[derive(Debug, Clone)]
struct Config {
    /// Рівень логування (`trace`|`debug`|`info`|`warn`|`error`).
    log_level: String,
    /// Необов’язковий шлях до файлу логів (якщо `None` — лог у stdout).
    log_path: Option<PathBuf>,
    /// Шлях до JSON із репозиторієм сніпетів.
    json_path: PathBuf,
}

impl Config {
    /// Створити конфіг з дефолтів і змін оточення.
    fn from_env_and_defaults(json_path: PathBuf) -> Self {
        let log_level = env::var("SNIPPETS_APP_LOG_LEVEL").unwrap_or_else(|_| "info".to_owned());
        let log_path = env::var("SNIPPETS_APP_LOG_PATH").ok().map(PathBuf::from);
        Self {
            log_level,
            log_path,
            json_path,
        }
    }
}

/// Ініціалізація `tracing`; повертає guard, якщо ведемо у файл.
fn init_tracing(cfg: &Config) -> Result<Option<WorkerGuard>> {
    let filter = EnvFilter::try_new(&cfg.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    if let Some(path) = cfg.log_path.as_ref() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("opening log file '{}'", path.display()))?;
        let (nb, guard) = tracing_appender::non_blocking(file);
        fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_level(true)
            .with_ansi(false)
            .with_writer(nb)
            .init();
        Ok(Some(guard))
    } else {
        fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_level(true)
            .init();
        Ok(None)
    }
}

/// Модель одного сніпету.
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Snippet {
    /// Назва (унікальний ключ у репозиторії).
    name: String,
    /// Мова/тип (наприклад, `rust`, `ts`, `text`).
    lang: String,
    /// Вміст сніпету.
    code: String,
    /// Час створення (UTC).
    created_at: DateTime<Utc>,
}

/// JSON-репозиторій сніпетів.
#[derive(Default, Serialize, Deserialize)]
struct Repo {
    /// Зберігання за ключем `name`.
    items: BTreeMap<String, Snippet>,
}

impl Repo {
    /// Завантажити репозиторій з файлу (або порожній, якщо файл відсутній).
    fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let file = File::open(path).with_context(|| format!("open '{}'", path.display()))?;
        let repo: Repo = serde_json::from_reader(io::BufReader::new(file))
            .with_context(|| format!("parse JSON '{}'", path.display()))?;
        Ok(repo)
    }

    /// Зберегти репозиторій у файл prettified-JSON.
    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = File::create(path).with_context(|| format!("create '{}'", path.display()))?;
        serde_json::to_writer_pretty(io::BufWriter::new(file), self)?;
        Ok(())
    }
}

/// CLI верхнього рівня.
#[derive(Parser, Debug)]
#[command(
    name = "snippets-app",
    version,
    about = "Store and manage code snippets; supports downloading via --download",
    disable_help_subcommand = true
)]
struct Cli {
    /// Шлях до JSON-сховища.
    #[arg(
        long = "json-path",
        value_name = "PATH",
        default_value = "snippets.json"
    )]
    json_path: PathBuf,

    /// Команда.
    #[command(subcommand)]
    cmd: Command,
}

/// Підкоманди.
#[derive(Subcommand, Debug)]
enum Command {
    /// Додати або оновити сніпет.
    Add(AddCmd),
    /// Вивести список сніпетів.
    List,
    /// Видалити сніпет за назвою.
    Remove(RemoveCmd),
}

/// Аргументи `add`.
#[derive(Args, Debug)]
struct AddCmd {
    /// Назва сніпету (ключ).
    #[arg(short = 'n', long = "name")]
    name: String,

    /// Мова/тип (наприклад, `rust`, `ts`, `text`).
    #[arg(short = 'l', long = "lang", default_value = "text")]
    lang: String,

    /// Завантажити вміст за URL (замість stdin).
    #[arg(long = "download", value_name = "URL")]
    download: Option<String>,
}

/// Аргументи `remove`.
#[derive(Args, Debug)]
struct RemoveCmd {
    /// Назва (ключ) для видалення.
    #[arg(short = 'n', long = "name")]
    name: String,
}

/// Прочитати увесь STDIN як текст (помилка, якщо порожньо).
fn read_stdin_text() -> Result<String> {
    let mut buf = Vec::new();
    io::stdin()
        .read_to_end(&mut buf)
        .context("read from STDIN")?;
    if buf.is_empty() {
        return Err(anyhow!(
            "STDIN is empty; pass --download <URL> or pipe content"
        ));
    }
    Ok(String::from_utf8_lossy(&buf).to_string())
}

/// Синхронно завантажити текст за URL.
fn fetch_url(url: &str) -> Result<String> {
    info!(target: "snippets", url = url, "downloading snippet");
    let response = reqwest::blocking::get(url).with_context(|| format!("GET {url}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("HTTP {status}"));
    }
    let text = response.text().context("read response body")?;
    Ok(text)
}

fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let cli = Cli::parse();
    let config = Config::from_env_and_defaults(cli.json_path.clone());
    let _guard = init_tracing(&config)?;

    debug!("Starting with json_path='{}'", config.json_path.display());

    let mut repo = Repo::load(&config.json_path)?;

    match cli.cmd {
        Command::List => {
            if repo.items.is_empty() {
                println!("(no snippets yet)");
                return Ok(());
            }
            for snippet in repo.items.values() {
                println!(
                    "{} [{}] @{}",
                    snippet.name,
                    snippet.lang,
                    snippet.created_at.to_rfc3339()
                );
            }
            info!(count = repo.items.len(), "listed snippets");
        }
        Command::Remove(RemoveCmd { name }) => {
            let existed = repo.items.remove(&name).is_some();
            repo.save(&config.json_path)?;
            if existed {
                println!("Removed '{name}'");
                info!(name = %name, "removed");
            } else {
                println!("No snippet named '{name}'");
                warn!(name = %name, "remove requested but not found");
            }
        }
        Command::Add(AddCmd {
            name,
            lang,
            download,
        }) => {
            let code = if let Some(url) = download.as_deref() {
                fetch_url(url)?
            } else {
                read_stdin_text()?
            };

            let snippet = Snippet {
                name: name.clone(),
                lang,
                code,
                created_at: Utc::now(),
            };
            repo.items.insert(name.clone(), snippet);
            repo.save(&config.json_path)?;
            println!("Saved '{name}'");
            info!(name = %name, "saved/updated");
        }
    }

    Ok(())
}
