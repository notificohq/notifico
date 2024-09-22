mod config;
pub mod engine;
pub mod recipients;
pub mod templater;

use crate::config::{Config, SimpleCredentials, SimplePipelineStorage};
use crate::engine::{Engine, PipelineRunner};
use clap::Parser;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use notifico_core::engine::EventContext;
use notifico_core::recipient::Recipient;
use notifico_smtp::EmailPlugin;
use notifico_telegram::TelegramPlugin;
use std::collections::HashMap;
use std::sync::Arc;
use templater::service::TemplaterService;
use tracing::{debug, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    event: String,

    #[arg(short, long)]
    context: String,

    #[arg(short, long)]
    recipient: Uuid,
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let config: Config = Figment::new()
        .merge(Yaml::file("notifico.yml"))
        .merge(Env::prefixed("NOTIFICO_"))
        .extract()
        .unwrap();

    debug!("Config: {:?}", config);

    let args = Args::parse();

    let event_context = args.context;
    let event_context: EventContext =
        serde_json::from_str(&event_context).expect("Failed to parse event context");

    info!("Received context: {:?}", event_context);

    let templater = Arc::new(TemplaterService::new("http://127.0.0.1:8000"));
    let credentials = Arc::new(SimpleCredentials::from_config(&config));
    let pipelines = Arc::new(SimplePipelineStorage::from_config(&config));

    let mut engine = Engine::new();

    engine.add_plugin(TelegramPlugin::new(templater.clone(), credentials.clone()));
    engine.add_plugin(EmailPlugin::new(templater, credentials));

    let recipinents: HashMap<Uuid, &Recipient> =
        HashMap::from_iter(config.projects[0].recipients.iter().map(|r| (r.id, r)));

    let runner = PipelineRunner {};
    runner
        .process_event(
            pipelines.clone(),
            config.projects[0].id,
            &args.event,
            event_context,
            engine,
            recipinents[&args.recipient].clone(),
        )
        .await
        .unwrap();
}
