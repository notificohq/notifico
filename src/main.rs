mod config;
mod event_handler;
mod http;
pub mod recipients;

use crate::config::{Config, SimpleCredentials, SimplePipelineStorage};
use actix::prelude::*;
use clap::Parser;
use event_handler::EventHandler;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use notifico_core::engine::Engine;
use notifico_core::templater::service::TemplaterService;
use notifico_smtp::EmailPlugin;
use notifico_telegram::TelegramPlugin;
use std::sync::Arc;
use tracing::debug;
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

#[actix::main]
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

    // let args = Args::parse();

    let templater = Arc::new(TemplaterService::new("http://127.0.0.1:8000"));
    let credentials = Arc::new(SimpleCredentials::from_config(&config));
    let pipelines = Arc::new(SimplePipelineStorage::from_config(&config));
    let directory = Arc::new(recipients::MemoryRecipientDirectory::new(
        config.projects[0].recipients.clone(),
    ));

    let mut engine = Engine::new();

    engine.add_plugin(TelegramPlugin::new(templater.clone(), credentials.clone()));
    engine.add_plugin(EmailPlugin::new(templater, credentials));

    let event_handler = EventHandler {
        pipeline_storage: pipelines.clone(),
        engine,
        recipient_storage: directory.clone(),
    }
    .start();

    tokio::spawn(http::start(event_handler.clone()));

    tokio::signal::ctrl_c().await.unwrap();
}
