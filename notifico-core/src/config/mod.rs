use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use url::Url;

pub mod credentials;
pub mod pipelines;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub http: Http,
    pub templates: PathBuf,
    pub credentials: PathBuf,
    pub pipelines: PathBuf,
    pub amqp: Amqp,
}

#[derive(Debug, Deserialize)]
pub struct Http {
    pub bind: SocketAddr,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Amqp {
    Bind { bind: SocketAddr },
    Broker { url: Url, queue: String },
}

impl Amqp {
    pub fn connection_url(&self) -> Url {
        match self {
            Self::Bind { bind } => format!("amqp://{}", bind).parse().unwrap(),
            Self::Broker { url, .. } => url.clone(),
        }
    }
}
