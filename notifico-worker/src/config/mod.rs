use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;

pub mod credentials;
pub mod pipelines;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub http: Http,
    pub templates: PathBuf,
    pub credentials: PathBuf,
    pub pipelines: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Http {
    pub bind: SocketAddr,
}
