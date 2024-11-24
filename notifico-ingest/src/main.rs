mod amqp;
mod http;

use crate::http::HttpExtensions;
use clap::Parser;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, env = "NOTIFICO_AMQP_URL")]
    amqp: Url,
    #[clap(
        long,
        env = "NOTIFICO_AMQP_WORKERS_ADDR",
        default_value = "notifico_workers"
    )]
    amqp_addr: String,
    #[clap(long, env = "NOTIFICO_HTTP_INGEST_BIND", default_value = "[::]:8000")]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let args = Args::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("Config: {:#?}", args);

    let (request_tx, request_rx) = flume::bounded(0);

    let ext = HttpExtensions { sender: request_tx };

    // Spawns HTTP servers and quits
    http::start(args.bind, ext).await;
    let amqp_client = tokio::spawn(amqp::run(args.amqp, args.amqp_addr, request_rx));
    amqp_client.await.unwrap();

    tokio::signal::ctrl_c().await.unwrap();
}
