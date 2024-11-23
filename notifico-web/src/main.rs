mod http;

use crate::http::HttpExtensions;
use clap::Parser;
use notifico_dbpipeline::DbPipelineStorage;
use notifico_project::ProjectController;
use notifico_subscription::SubscriptionManager;
use notifico_template::db::DbTemplateSource;
use sea_orm::{ConnectOptions, Database};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, log};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, env = "NOTIFICO_DB_URL")]
    db_url: Url,
    #[clap(long, env = "NOTIFICO_SECRET_KEY")]
    secret_key: String,
    #[clap(long, env = "NOTIFICO_WEB_BIND", default_value = "[::]:8000")]
    bind: SocketAddr,
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

    let mut db_conn_options = ConnectOptions::new(args.db_url.to_string());
    db_conn_options.sqlx_logging_level(log::LevelFilter::Debug);

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

    let templates = Arc::new(DbTemplateSource::new(db_connection.clone())); // Implement your template source here
    templates.setup().await.unwrap();

    let ext = HttpExtensions {
        projects_controller: projects,
        subman,
        pipeline_storage,
        templates_controller: templates,
    };

    // Spawns HTTP servers and quits
    http::start(args.bind, ext).await;

    tokio::signal::ctrl_c().await.unwrap();
}
