mod http;

use crate::http::HttpExtensions;
use clap::Parser;
use notifico_core::db::create_sqlite_if_not_exists;
use notifico_core::http::SecretKey;
use notifico_subscription::SubscriptionManager;
use sea_orm::{ConnectOptions, Database};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, log};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, env = "NOTIFICO_DB")]
    db_url: Url,
    #[clap(long, env = "NOTIFICO_SECRET_KEY")]
    secret_key: String,
    #[clap(long, env = "NOTIFICO_USERAPI_BIND", default_value = "[::]:8000")]
    bind: SocketAddr,
    #[clap(long, env = "NOTIFICO_USERAPI_URL")]
    userapi_url: Url,
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

    create_sqlite_if_not_exists(&args.db_url);

    let mut db_conn_options = ConnectOptions::new(args.db_url.to_string());
    db_conn_options.sqlx_logging_level(log::LevelFilter::Debug);

    let db_connection = Database::connect(db_conn_options).await.unwrap();

    // Initializing plugins
    let subman = Arc::new(SubscriptionManager::new(
        db_connection.clone(),
        args.secret_key.as_bytes().to_vec(),
        args.userapi_url,
    ));
    subman.setup().await.unwrap();

    let ext = HttpExtensions {
        subman,
        secret_key: Arc::new(SecretKey(args.secret_key.as_bytes().to_vec())),
    };

    // Spawns HTTP servers and quits
    http::start(args.bind, ext).await;

    tokio::signal::ctrl_c().await.unwrap();
}
