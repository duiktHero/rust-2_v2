use clap::{ArgAction, Parser};
use config::{Config as RawConfig, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::Path, process::ExitCode};

#[derive(Parser, Debug)]
#[command(
    name = "task_3_9",
    version,
    about = "Prints its configuration to STDOUT",
    disable_help_subcommand = true
)]
struct Cli {
    #[arg(short = 'd', long = "debug", action = ArgAction::SetTrue)]
    debug: bool,

    #[arg(
        short = 'c',
        long = "conf",
        value_name = "CONF",
        env = "CONF_FILE",
        default_value = "config.toml"
    )]
    conf: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    debug: bool,
    server: Server,
    db: Db,
    log: Log,
    background: Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Server {
    external_url: String,
    http_port: u16,
    grpc_port: u16,
    healthz_port: u16,
    metrics_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Db {
    mysql: MySql,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MySql {
    host: String,
    port: u16,
    dating: String,
    user: String,
    pass: String,
    connections: Connections,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Connections {
    max_idle: u32,
    max_open: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Log {
    app: AppLog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppLog {
    level: LogLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Background {
    watchdog: Watchdog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Watchdog {
    period: String,
    limit: u32,
    lock_timeout: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            debug: false,
            server: Server::default(),
            db: Db::default(),
            log: Log::default(),
            background: Background::default(),
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self {
            external_url: "http://127.0.0.1".into(),
            http_port: 8081,
            grpc_port: 8082,
            healthz_port: 10025,
            metrics_port: 9199,
        }
    }
}

impl Default for Db {
    fn default() -> Self {
        Self { mysql: MySql::default() }
    }
}

impl Default for MySql {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3306,
            dating: "default".into(),
            user: "root".into(),
            pass: "".into(),
            connections: Connections::default(),
        }
    }
}

impl Default for Connections {
    fn default() -> Self {
        Self { max_idle: 30, max_open: 30 }
    }
}

impl Default for Log {
    fn default() -> Self {
        Self { app: AppLog::default() }
    }
}

impl Default for AppLog {
    fn default() -> Self {
        Self { level: LogLevel::Info }
    }
}

impl Default for Background {
    fn default() -> Self {
        Self { watchdog: Watchdog::default() }
    }
}

impl Default for Watchdog {
    fn default() -> Self {
        Self {
            period: "5s".into(),
            limit: 10,
            lock_timeout: "4s".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        };
        f.write_str(s)
    }
}

fn build_config(cli: &Cli) -> Result<Config, ConfigError> {
    let mut builder = RawConfig::builder()
        .set_default("debug", Config::default().debug)?
        .set_default("server.external_url", Server::default().external_url)?
        .set_default("server.http_port", Server::default().http_port)?
        .set_default("server.grpc_port", Server::default().grpc_port)?
        .set_default("server.healthz_port", Server::default().healthz_port)?
        .set_default("server.metrics_port", Server::default().metrics_port)?
        .set_default("db.mysql.host", MySql::default().host)?
        .set_default("db.mysql.port", MySql::default().port)?
        .set_default("db.mysql.dating", MySql::default().dating)?
        .set_default("db.mysql.user", MySql::default().user)?
        .set_default("db.mysql.pass", MySql::default().pass)?
        .set_default("db.mysql.connections.max_idle", Connections::default().max_idle)?
        .set_default("db.mysql.connections.max_open", Connections::default().max_open)?
        .set_default("log.app.level", AppLog::default().level.to_string())?
        .set_default("background.watchdog.period", Watchdog::default().period)?
        .set_default("background.watchdog.limit", Watchdog::default().limit)?
        .set_default("background.watchdog.lock_timeout", Watchdog::default().lock_timeout)?;

    let conf_path = Path::new(&cli.conf);
    if conf_path.exists() {
        builder = builder.add_source(File::from(conf_path).required(false));
    }

    builder = builder.add_source(
        Environment::with_prefix("CONF")
            .separator("__")
            .list_separator(","),
    );

    let mut cfg: Config = builder.build()?.try_deserialize()?;

    if cli.debug {
        cfg.debug = true;
    }

    Ok(cfg)
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match build_config(&cli) {
        Ok(cfg) => {
            println!("{}", serde_json::to_string_pretty(&cfg).unwrap());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {e}");
            ExitCode::from(1)
        }
    }
}
