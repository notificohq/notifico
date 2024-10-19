mod config;
mod event_handler;
mod http;

use crate::config::Config;
use actix::prelude::*;
use clap::Parser;
use config::credentials::SimpleCredentials;
use config::pipelines::SimplePipelineStorage;
use event_handler::EventHandler;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use notifico_core::engine::Engine;
use notifico_smpp::SmppPlugin;
use notifico_smtp::EmailPlugin;
use notifico_telegram::TelegramPlugin;
use notifico_template::LocalTemplater;
use notifico_whatsapp::WaBusinessPlugin;
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

    let credentials = Arc::new(SimpleCredentials::from_config(&config));
    let pipelines = Arc::new(SimplePipelineStorage::from_config(&config));

    // Create Engine with plugins
    let mut engine = Engine::new();
    engine.add_plugin(Arc::new(LocalTemplater::new(&config.templates.path)));

    engine.add_plugin(Arc::new(TelegramPlugin::new(credentials.clone())));
    engine.add_plugin(Arc::new(EmailPlugin::new(credentials.clone())));
    engine.add_plugin(Arc::new(WaBusinessPlugin::new(credentials.clone())));
    engine.add_plugin(Arc::new(SmppPlugin::new(credentials.clone())));

    let event_handler = EventHandler {
        pipeline_storage: pipelines.clone(),
        engine,
    }
    .start();

    tokio::spawn(http::start(event_handler.clone(), config.http.bind));

    tokio::signal::ctrl_c().await.unwrap();
}
