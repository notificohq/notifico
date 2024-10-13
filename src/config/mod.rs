use notifico_core::credentials::Credential;
use notifico_core::pipeline::Pipeline;
use notifico_core::recipient::Recipient;
use serde::Deserialize;
use std::net::SocketAddr;
use url::Url;
use uuid::Uuid;

pub mod credentials;
pub mod pipelines;
pub mod recipients;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub projects: Vec<Project>,
    pub secret_key: String,
    pub http: Http,
    pub db: Database,
}

#[derive(Debug, Deserialize)]
pub struct Http {
    pub bind: SocketAddr,
    pub subscriber_url: Url,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub url: Url,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(default = "Uuid::nil")]
    pub id: Uuid,
    pub pipelines: Vec<Pipeline>,
    pub credentials: Vec<Credential>,
    pub recipients: Vec<Recipient>,
}
