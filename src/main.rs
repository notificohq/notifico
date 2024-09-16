mod config;
pub mod engine;
pub mod recipients;
pub mod templater;

use crate::config::{Config, SimpleCredentials};
use crate::engine::Engine;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use notifico_core::engine::{EventContext, PipelineContext};
use notifico_telegram::TelegramPlugin;
use std::env::args;
use std::sync::Arc;
use templater::service::TemplaterService;
use tracing::{debug, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

    let event_context = args()
        .nth(1)
        .expect("Please provide an event context as the first argument");
    let event_context: EventContext =
        serde_json::from_str(&event_context).expect("Failed to parse event context");

    info!("Received context: {:?}", event_context);

    let templater = Arc::new(TemplaterService::new("http://127.0.0.1:8000"));
    let credentials = Arc::new(SimpleCredentials::new(config.credentials.clone()));

    let mut engine = Engine::new();

    let tg_plugin = TelegramPlugin::new(templater, credentials);
    engine.add_plugin(tg_plugin);

    // Pipeline;
    {
        let mut context = PipelineContext::default();
        context.recipient = config.recipients.get(0).cloned();
        context.event_context = event_context;

        for step in config.pipelines[0].steps.iter() {
            engine.execute_step(&mut context, step).await.unwrap()
        }
    }
}
