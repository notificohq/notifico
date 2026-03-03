use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub queue: QueueConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub project: ProjectConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_mode")]
    pub mode: ServerMode,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_admin_port")]
    pub admin_port: u16,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerMode {
    All,
    Api,
    Worker,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_backend")]
    pub backend: String,
    #[serde(default = "default_db_url")]
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    #[serde(default = "default_queue_backend")]
    pub backend: String,
    #[serde(default)]
    pub redis_url: Option<String>,
    #[serde(default)]
    pub amqp_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_storage_backend")]
    pub backend: String,
    #[serde(default = "default_storage_path")]
    pub path: String,
    #[serde(default)]
    pub s3_bucket: Option<String>,
    #[serde(default)]
    pub s3_endpoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub encryption_key: Option<String>,
    #[serde(default)]
    pub jwt_secret: Option<String>,
    #[serde(default)]
    pub oidc: Option<OidcConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    pub enabled: bool,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    #[serde(default = "default_locale")]
    pub default_locale: String,
}

fn default_mode() -> ServerMode {
    ServerMode::All
}
fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    8000
}
fn default_admin_port() -> u16 {
    8001
}
fn default_db_backend() -> String {
    "sqlite".into()
}
fn default_db_url() -> String {
    "sqlite://notifico.db?mode=rwc".into()
}
fn default_queue_backend() -> String {
    "redis".into()
}
fn default_storage_backend() -> String {
    "filesystem".into()
}
fn default_storage_path() -> String {
    "./data/assets".into()
}
fn default_locale() -> String {
    "en".into()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            host: default_host(),
            port: default_port(),
            admin_port: default_admin_port(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            backend: default_db_backend(),
            url: default_db_url(),
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            backend: default_queue_backend(),
            redis_url: None,
            amqp_url: None,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: default_storage_backend(),
            path: default_storage_path(),
            s3_bucket: None,
            s3_endpoint: None,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            encryption_key: None,
            jwt_secret: None,
            oidc: None,
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            default_locale: default_locale(),
        }
    }
}

impl Config {
    /// Load config from notifico.toml (optional) + NOTIFICO_ env vars.
    pub fn load(config_path: Option<&str>) -> Result<Self, figment::Error> {
        let mut figment = Figment::new();

        if let Some(path) = config_path {
            figment = figment.merge(Toml::file(path));
        } else {
            figment = figment.merge(Toml::file("notifico.toml").nested());
        }

        figment = figment.merge(Env::prefixed("NOTIFICO_").split("_"));

        figment.extract()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_loads() {
        let config = Config::load(None).unwrap();
        assert_eq!(config.server.mode, ServerMode::All);
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.server.admin_port, 8001);
        assert_eq!(config.database.backend, "sqlite");
        assert_eq!(config.project.default_locale, "en");
        assert_eq!(config.queue.backend, "redis");
        assert_eq!(config.storage.backend, "filesystem");
    }

    #[test]
    fn config_from_toml_string() {
        let toml_str = r#"
            [server]
            mode = "worker"
            port = 9000

            [project]
            default_locale = "ru"
        "#;

        let config: Config = Figment::new()
            .merge(Toml::string(toml_str))
            .extract()
            .unwrap();

        assert_eq!(config.server.mode, ServerMode::Worker);
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.server.admin_port, 8001); // default
        assert_eq!(config.project.default_locale, "ru");
    }
}
