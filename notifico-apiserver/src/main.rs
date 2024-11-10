mod amqp;
mod http;

use crate::http::HttpExtensions;
use clap::Parser;
use notifico_core::http::SecretKey;
use notifico_dbpipeline::DbPipelineStorage;
use notifico_project::ProjectController;
use notifico_subscription::SubscriptionManager;
use sea_orm::{ConnectOptions, Database};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, env = "NOTIFICO_DB_URL")]
    db_url: Url,
    #[clap(long, env = "NOTIFICO_SECRET_KEY")]
    secret_key: String,
    #[clap(long, env = "NOTIFICO_AMQP_URL")]
    amqp: Url,
    #[clap(
        long,
        env = "NOTIFICO_AMQP_WORKERS_ADDR",
        default_value = "notifico_workers"
    )]
    amqp_addr: String,
    #[clap(long, env = "NOTIFICO_SERVICE_API_BIND")]
    service_api_bind: SocketAddr,
    #[clap(long, env = "NOTIFICO_CLIENT_API_BIND")]
    client_api_bind: SocketAddr,
    #[clap(long, env = "NOTIFICO_CLIENT_API_URL")]
    client_api_url: Url,
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

    let db_conn_options = ConnectOptions::new(args.db_url.to_string());
    let db_connection = Database::connect(db_conn_options).await.unwrap();

    // Initializing plugins
    let subman = Arc::new(SubscriptionManager::new(
        db_connection.clone(),
        args.secret_key.as_bytes().to_vec(),
        args.client_api_url,
    ));
    subman.setup().await.unwrap();

    let pipeline_storage = Arc::new(DbPipelineStorage::new(db_connection.clone()));
    pipeline_storage.setup().await.unwrap();

    let projects = Arc::new(ProjectController::new(db_connection.clone()));
    projects.setup().await.unwrap();

    let (request_tx, request_rx) = tokio::sync::mpsc::channel(1);

    let ext = HttpExtensions {
        projects_controller: projects,
        sender: request_tx,
        subman,
        secret_key: Arc::new(SecretKey(args.secret_key.as_bytes().to_vec())),
        pipeline_storage,
    };

    tokio::spawn(http::start(
        args.service_api_bind,
        args.client_api_bind,
        ext,
    ));
    tokio::spawn(amqp::run(args.amqp, args.amqp_addr, request_rx));

    tokio::signal::ctrl_c().await.unwrap();
}
