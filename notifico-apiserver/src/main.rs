mod amqp;
mod http;

use crate::http::HttpExtensions;
use clap::Parser;
use figment::providers::Toml;
use figment::{
    providers::{Env, Format},
    Figment,
};
use notifico_core::config::Config;
use notifico_subscription::SubscriptionManager;
use sea_orm::{ConnectOptions, Database};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser)]
struct Args {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(|| "notifico.toml".into());

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let config: Config = Figment::new()
        .merge(Toml::file(config_path))
        .merge(Env::prefixed("NOTIFICO_"))
        .extract()
        .unwrap();

    info!("Config: {:#?}", config);

    let (request_tx, request_rx) = tokio::sync::mpsc::channel(1);

    let db_conn_options = ConnectOptions::new(config.db.url.to_string());
    let db_connection = Database::connect(db_conn_options).await.unwrap();

    // Initializing plugins
    let subman = Arc::new(SubscriptionManager::new(
        db_connection,
        config.secret_key.as_bytes().to_vec(),
        config.external_url,
    ));
    subman.setup().await.unwrap();

    let ext = HttpExtensions {
        sender: request_tx,
        subman,
    };

    tokio::spawn(http::start(config.http.bind, ext));
    tokio::spawn(amqp::run(config.amqp, request_rx));

    tokio::signal::ctrl_c().await.unwrap();
}
