mod amqp;

use crate::amqp::AmqpClient;
use clap::Parser;
use figment::{providers::Format, providers::Toml, Figment};
use log::debug;
use notifico_core::config::credentials::MemoryCredentialStorage;
use notifico_core::db::create_sqlite_if_not_exists;
use notifico_core::engine::plugin::core::CorePlugin;
use notifico_core::engine::Engine;
use notifico_core::pipeline::event::EventHandler;
use notifico_core::pipeline::executor::PipelineExecutor;
use notifico_core::queue::ReceiverChannel;
use notifico_core::recorder::BaseRecorder;
use notifico_core::transport::TransportRegistry;
use notifico_dbpipeline::DbPipelineStorage;
use notifico_subscription::SubscriptionManager;
use notifico_template::db::DbTemplateSource;
use notifico_template::Templater;
use notifico_transports::all_transports;
use sea_orm::{ConnectOptions, Database};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, env = "NOTIFICO_DB")]
    db_url: Url,
    #[clap(long, env = "NOTIFICO_SECRET_KEY")]
    secret_key: String,
    #[clap(long, env = "NOTIFICO_USERAPI_URL")]
    userapi_url: Url,
    #[clap(long, env = "NOTIFICO_AMQP_URL")]
    amqp_url: Url,
    #[clap(
        long,
        env = "NOTIFICO_AMQP_WORKERS_ADDR",
        default_value = "notifico_workers"
    )]
    amqp_addr: String,

    #[clap(
        long,
        env = "NOTIFICO_CREDENTIALS_PATH",
        default_value = "/var/lib/notifico/credentials.toml"
    )]
    credentials_path: PathBuf,
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

    let credentials = {
        let credential_config: serde_json::Value = {
            let mut config = Figment::new().merge(Toml::file(args.credentials_path));
            if let Ok(env_credentials) = std::env::var("NOTIFICO_CREDENTIALS") {
                config = config.merge(Toml::string(&env_credentials));
            }
            config.extract().unwrap()
        };
        Arc::new(MemoryCredentialStorage::from_config(credential_config).unwrap())
    };
    let pipelines = Arc::new(DbPipelineStorage::new(db_connection.clone()));

    // Initialize AMQP client
    let mut amqp_client = AmqpClient::connect(args.amqp_url, "wrk".to_string())
        .await
        .unwrap();

    let (pipelines_tx, pipelines_rx) = amqp_client
        .channel("pipelines", "pipeline-link")
        .await
        .unwrap();
    let pipelines_tx = Arc::new(pipelines_tx);
    let events_rx = amqp_client
        .create_receiver("notifico_workers", "event-link")
        .await
        .unwrap();

    // Create Engine with plugins
    let mut engine = Engine::new();

    let recorder = Arc::new(BaseRecorder::new());

    engine.add_plugin(Arc::new(CorePlugin::new(pipelines_tx.clone())));

    let templater_source = Arc::new(DbTemplateSource::new(db_connection.clone()));
    engine.add_plugin(Arc::new(Templater::new(templater_source.clone())));

    let mut transport_registry = TransportRegistry::new();
    for (engine_plugin, transport_plugin) in all_transports(credentials.clone(), recorder.clone()) {
        engine.add_plugin(engine_plugin);
        transport_registry.register(transport_plugin);
    }

    let subman = Arc::new(SubscriptionManager::new(
        db_connection,
        args.secret_key.as_bytes().to_vec(),
        args.userapi_url,
    ));
    engine.add_plugin(subman.clone());

    // Setup stateful plugins
    pipelines.setup().await.unwrap();
    templater_source.setup().await.unwrap();
    subman.setup().await.unwrap();

    let executor = Arc::new(PipelineExecutor::new(engine));
    let event_handler = EventHandler::new(pipelines.clone(), pipelines_tx.clone());

    loop {
        tokio::select! {
            Ok(task) = pipelines_rx.receive() => {
                debug!("Received pipeline: {:?}", task);
                let executor = executor.clone();
                let _handle = tokio::spawn(async move {executor.execute_pipeline(serde_json::from_str(&task).unwrap()).await});
            }
            Ok(event) = events_rx.receive() => {
                debug!("Received event: {:?}", event);
                event_handler.process_eventrequest(serde_json::from_str(&event).unwrap()).await.unwrap();
            }
            _ = tokio::signal::ctrl_c() => break,
        }
    }
}
