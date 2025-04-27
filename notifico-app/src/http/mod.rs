pub mod auth;
pub mod ingest;
pub mod metrics;
pub mod public;
pub mod ui;

#[derive(Clone)]
pub struct SecretKey(pub Vec<u8>);
