mod amqp;

use clap::Parser;
use figment::{providers::Format, providers::Toml, Figment};
use notifico_core::config::credentials::MemoryCredentialStorage;
use notifico_core::db::create_sqlite_if_not_exists;
use notifico_core::engine::Engine;
use notifico_core::pipeline::runner::PipelineRunner;
use notifico_core::recorder::BaseRecorder;
use notifico_dbpipeline::DbPipelineStorage;
use notifico_slack::SlackPlugin;
use notifico_smpp::SmppPlugin;
use notifico_smtp::EmailPlugin;
use notifico_subscription::SubscriptionManager;
use notifico_telegram::TelegramPlugin;
use notifico_template::db::DbTemplateSource;
use notifico_template::Templater;
use notifico_whatsapp::WaBusinessPlugin;
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

    // Create Engine with plugins
    let mut engine = Engine::new();

    let recorder = Arc::new(BaseRecorder::new());

    let templater_source = Arc::new(DbTemplateSource::new(db_connection.clone()));
    engine.add_plugin(Arc::new(Templater::new(templater_source.clone())));

    engine.add_plugin(Arc::new(TelegramPlugin::new(
        credentials.clone(),
        recorder.clone(),
    )));
    engine.add_plugin(Arc::new(EmailPlugin::new(
        credentials.clone(),
        recorder.clone(),
    )));
    engine.add_plugin(Arc::new(WaBusinessPlugin::new(
        credentials.clone(),
        recorder.clone(),
    )));
    engine.add_plugin(Arc::new(SmppPlugin::new(credentials.clone())));
    engine.add_plugin(Arc::new(SlackPlugin::new(
        credentials.clone(),
        recorder.clone(),
    )));

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

    // Create PipelineRunner, the core component of the Notifico system
    let runner = Arc::new(PipelineRunner::new(pipelines.clone(), engine));

    tokio::spawn(amqp::start(runner.clone(), args.amqp_url, args.amqp_addr));

    tokio::signal::ctrl_c().await.unwrap();
}
