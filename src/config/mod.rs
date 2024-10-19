use notifico_core::credentials::Credential;
use notifico_core::pipeline::Pipeline;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use uuid::Uuid;

pub mod credentials;
pub mod pipelines;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub projects: Vec<Project>,
    pub http: Http,
    pub templates: TemplatesConfig,
}

#[derive(Debug, Deserialize)]
pub struct Http {
    pub bind: SocketAddr,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TemplatesConfig {
    pub(crate) path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(default = "Uuid::nil")]
    pub id: Uuid,
    pub pipelines: Vec<Pipeline>,
    pub credentials: Vec<Credential>,
}
