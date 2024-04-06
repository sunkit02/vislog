use config::{Config, ConfigError, File, FileFormat};
use serde::Deserialize;

pub const CONFIG_FILE_PATH: &str = "./vislog-configs.toml";

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub server: Server,
    pub log: Log,
    pub data: Data,
    pub fetching: Fetching,
}

impl ServerConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::new(CONFIG_FILE_PATH, FileFormat::Toml))
            .build()?;

        Ok(s.try_deserialize()?)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        let server = Server {
            host: "127.0.0.1".to_owned(),
            port: 8080,
        };

        let data = Data {
            storage: "../data".to_owned(),
            all_programs_file: "programs.json".to_owned(),
        };

        let log = Log {
            level: Some(LogLevel::Info),
            with_target: Some(true),
        };

        let fetching = Fetching { url: "https://iq5prod1.smartcatalogiq.com/apis/progAPI?path=/sitecore/content/Catalogs/Union-University/2023/Academic-Catalogue-Undergraduate-Catalogue&format=json".to_owned() };

        Self {
            server,
            data,
            log,
            fetching,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Data {
    pub storage: String,
    pub all_programs_file: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Log {
    pub level: Option<LogLevel>,
    pub with_target: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub enum LogLevel {
    Trace,
    Warn,
    Info,
    Error,
}

impl AsRef<str> for LogLevel {
    fn as_ref(&self) -> &str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Error => "error",
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Fetching {
    pub url: String,
}
