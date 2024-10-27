mod amqp;
mod http;

use clap::Parser;
use fe2o3_amqp::{Connection, Sender, Session};
use figment::providers::Toml;
use figment::{
    providers::{Env, Format},
    Figment,
};
use notifico_core::config::{Amqp, Config};
use std::path::PathBuf;
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

    // AMQP sender

    let (request_tx, mut request_rx) = tokio::sync::mpsc::channel(1);

    tokio::spawn(http::start(config.http.bind, request_tx));
    tokio::spawn(amqp::run(config.amqp, request_rx));

    tokio::signal::ctrl_c().await.unwrap();
}
