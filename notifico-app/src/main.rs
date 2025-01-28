mod amqp;
mod controllers;
mod crud_table;
#[allow(unused_imports)]
pub(crate) mod entity;
mod http;
mod plugin;

use crate::amqp::AmqpClient;
use crate::controllers::event::EventDbController;
use crate::controllers::group::GroupDbController;
use crate::controllers::pipeline::PipelineDbController;
use crate::controllers::project::ProjectController;
use crate::controllers::recipient::RecipientDbController;
use crate::controllers::subscription::SubscriptionDbController;
use crate::controllers::template::DbTemplateSource;
use crate::http::ingest::HttpIngestExtensions;
use crate::http::management::HttpManagementExtensions;
use crate::http::public::HttpPublicExtensions;
use crate::plugin::SubscriptionPlugin;
use clap::{Parser, Subcommand};
use migration::{Migrator, MigratorTrait};
use notifico_attachment::AttachmentPlugin;
use notifico_core::credentials::env::EnvCredentialStorage;
use notifico_core::engine::plugin::core::CorePlugin;
use notifico_core::engine::Engine;
use notifico_core::http::SecretKey;
use notifico_core::pipeline::event::EventHandler;
use notifico_core::pipeline::executor::PipelineExecutor;
use notifico_core::queue::{ReceiverChannel, SenderChannel};
use notifico_core::recorder::BaseRecorder;
use notifico_core::transport::TransportRegistry;
use notifico_template::Templater;
use notifico_transports::all_transports;
use sea_orm::{ConnectOptions, Database};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use url::Url;

const COMPONENT_WORKER: &str = "worker";
const COMPONENT_MANAGEMENT: &str = "management";
const COMPONENT_INGEST: &str = "ingest";
const COMPONENT_PUBLIC: &str = "public";

const WEAK_SECRET_KEY: &str = "weak-secret-key";

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, env = "NOTIFICO_DB")]
    db: Url,
    #[clap(long, env = "NOTIFICO_SECRET_KEY", default_value = WEAK_SECRET_KEY)]
    secret_key: String,
    #[clap(long, env = "NOTIFICO_PUBLIC_URL")]
    public_url: Option<Url>,
    #[clap(long, env = "NOTIFICO_AMQP")]
    amqp: Option<Url>,
    #[clap(long, env = "NOTIFICO_AMQP_PREFIX", default_value = "notifico_")]
    amqp_prefix: String,

    #[clap(long, env = "NOTIFICO_UI_BIND", default_value = "[::]:8000")]
    management: SocketAddr,
    #[clap(long, env = "NOTIFICO_HTTP_INGEST_BIND", default_value = "[::]:8001")]
    ingest: SocketAddr,
    #[clap(long, env = "NOTIFICO_PUBLIC_BIND", default_value = "[::]:8002")]
    public: SocketAddr,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run { components: Vec<String> },
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "h2=warn,info");
    }

    let args = Args::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    debug!("Config: {:#?}", args);

    match args.command {
        Commands::Run { components } => {
            if args.secret_key == WEAK_SECRET_KEY {
                warn!("Weak secret key is not recommended for production environments. Please set NOTIFICO_SECRET_KEY to a stronger key.");
            }
            if args.public_url.is_none() {
                warn!("NOTIFICO_PUBLIC_URL is not provided. Some features may not work: List-Unsubscribe");
            }

            let components: HashSet<String> = components.into_iter().collect();

            // Initialize channels
            let (pipelines_tx, pipelines_rx) = flume::unbounded();
            let mut pipelines_tx: Arc<dyn SenderChannel> = Arc::new(pipelines_tx);
            let mut pipelines_rx: Arc<dyn ReceiverChannel> = Arc::new(pipelines_rx);

            let (events_tx, events_rx) = flume::bounded(1);
            let mut events_tx: Arc<dyn SenderChannel> = Arc::new(events_tx);
            let mut events_rx: Arc<dyn ReceiverChannel> = Arc::new(events_rx);

            #[allow(unused_assignments)]
            let mut amqp_client = None;
            if let Some(amqp_url) = args.amqp {
                // Initialize AMQP client and replace local channels with AMQP ones
                amqp_client = Some(
                    AmqpClient::connect(amqp_url, "wrk".to_string())
                        .await
                        .unwrap(),
                );

                let (amqp_pipelines_tx, amqp_pipelines_rx) = amqp_client
                    .as_mut()
                    .unwrap()
                    .channel(&(args.amqp_prefix.clone() + "pipelines"), "pipeline-link")
                    .await
                    .unwrap();

                pipelines_tx = Arc::new(amqp_pipelines_tx);
                pipelines_rx = Arc::new(amqp_pipelines_rx);

                let (amqp_events_tx, amqp_events_rx) = amqp_client
                    .as_mut()
                    .unwrap()
                    .channel(&(args.amqp_prefix.clone() + "events"), "event-link")
                    .await
                    .unwrap();
                events_tx = Arc::new(amqp_events_tx);
                events_rx = Arc::new(amqp_events_rx);
            }

            // Initialize db connection
            create_sqlite_if_not_exists(&args.db);
            let mut db_conn_options = ConnectOptions::new(args.db.to_string());
            db_conn_options.sqlx_logging_level(log::LevelFilter::Debug);
            let db_connection = Database::connect(db_conn_options).await.unwrap();

            Migrator::up(&db_connection, None).await.unwrap();

            // Storages
            let credentials = Arc::new(EnvCredentialStorage::new());
            let pipeline_controller = Arc::new(PipelineDbController::new(db_connection.clone()));
            let recorder = Arc::new(BaseRecorder::new());
            let templater_source = Arc::new(DbTemplateSource::new(db_connection.clone()));
            let recipient_controller = Arc::new(RecipientDbController::new(db_connection.clone()));

            let subscription_controller =
                Arc::new(SubscriptionDbController::new(db_connection.clone()));

            let subman = Arc::new(SubscriptionPlugin::new(
                subscription_controller.clone(),
                args.secret_key.as_bytes().to_vec(),
                args.public_url,
            ));

            let secret_key = Arc::new(SecretKey(args.secret_key.as_bytes().to_vec()));

            // Create Engine with plugins
            let mut transport_registry = TransportRegistry::new();

            let engine = {
                let mut engine = Engine::new();
                engine.add_plugin(Arc::new(CorePlugin::new(
                    pipelines_tx.clone(),
                    recipient_controller.clone(),
                )));
                engine.add_plugin(Arc::new(Templater::new(templater_source.clone())));
                engine.add_plugin(subman.clone());

                let attachment_plugin = Arc::new(AttachmentPlugin::new(false));
                engine.add_plugin(attachment_plugin.clone());

                for (engine_plugin, transport) in all_transports(
                    credentials.clone(),
                    recorder.clone(),
                    attachment_plugin.clone(),
                ) {
                    engine.add_plugin(engine_plugin);
                    transport_registry.register(transport);
                }
                engine
            };

            let transport_registry = Arc::new(transport_registry);

            // Spawn HTTP servers
            if components.is_empty() || components.contains(COMPONENT_INGEST) {
                info!("Starting HTTP ingest server on {}", args.ingest);
                let ext = HttpIngestExtensions { sender: events_tx };

                http::ingest::start(args.ingest, ext).await;
            }

            if components.is_empty() || components.contains(COMPONENT_PUBLIC) {
                info!("Starting HTTP public server on {}", args.public);
                let ext = HttpPublicExtensions {
                    subscription_controller: subscription_controller.clone(),
                    secret_key: secret_key.clone(),
                };
                http::public::start(args.public, ext).await;
            }

            if components.is_empty() || components.contains(COMPONENT_MANAGEMENT) {
                let event_controller = Arc::new(EventDbController::new(db_connection.clone()));
                let project_controller = Arc::new(ProjectController::new(db_connection.clone()));
                let group_controller = Arc::new(GroupDbController::new(db_connection.clone()));

                info!("Starting HTTP management server on {}", args.management);
                let ext = HttpManagementExtensions {
                    recipient_controller: recipient_controller.clone(),
                    project_controller,
                    subscription_controller: subscription_controller.clone(),
                    pipeline_controller: pipeline_controller.clone(),
                    template_controller: templater_source.clone(),
                    event_controller,
                    group_controller,
                    secret_key: secret_key.clone(),
                    credential_controller: credentials.clone(),
                    transport_registry,
                };

                http::management::start(args.management, ext).await;
            }

            if components.is_empty() || components.contains(COMPONENT_WORKER) {
                // Main loop
                let executor = Arc::new(PipelineExecutor::new(engine));
                let event_handler = Arc::new(EventHandler::new(
                    pipeline_controller.clone(),
                    pipelines_tx.clone(),
                ));

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            Ok(task) = pipelines_rx.receive() => {
                                debug!("Received pipeline: {:?}", task);
                                let executor = executor.clone();
                                let _handle = tokio::spawn(async move {executor.execute_pipeline(serde_json::from_str(&task).unwrap()).await});
                            }
                            Ok(event) = events_rx.receive() => {
                                debug!("Received event: {:?}", event);
                                let event_handler = event_handler.clone();
                                let _handle = tokio::spawn(async move {event_handler.process_eventrequest(serde_json::from_str(&event).unwrap()).await.unwrap()});
                            }
                        }
                    }
                });
            }

            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

fn create_sqlite_if_not_exists(db_url: &Url) {
    if db_url.scheme() == "sqlite" {
        let url_string = db_url.to_string();
        let file: Vec<&str> = url_string
            .trim_start_matches("sqlite://")
            .split("?")
            .collect();
        let _ = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(file[0]);
    }
}
