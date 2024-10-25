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

    // AMQP sender
    let mut connection = Connection::open("connection-1", config.amqp.connection_url())
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    let queue_name = match config.amqp {
        Amqp::Bind { .. } => String::default(),
        Amqp::Broker { queue, .. } => queue,
    };

    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", queue_name)
        .await
        .unwrap();
    //

    let (request_tx, mut request_rx) = tokio::sync::mpsc::channel(1);

    tokio::spawn(http::start(config.http.bind, request_tx));

    loop {
        tokio::select! {
            req = request_rx.recv() => {
                info!("Sending message");
                let msg = serde_json::to_string(&req).unwrap();
                let _outcome = sender.send(msg).await.unwrap();
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down");
                break;
            }
        }
    }
}
