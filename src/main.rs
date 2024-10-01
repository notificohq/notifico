mod config;
mod event_handler;
mod http;
pub mod recipients;

use crate::config::{Config, SimpleCredentials, SimplePipelineStorage};
use crate::recipients::MemoryRecipientDirectory;
use actix::prelude::*;
use clap::Parser;
use event_handler::EventHandler;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use hmac::{Hmac, Mac};
use notifico_core::engine::Engine;
use notifico_core::templater::service::TemplaterService;
use notifico_smtp::EmailPlugin;
use notifico_subscription::SubscriptionManager;
use notifico_telegram::TelegramPlugin;
use sea_orm::{ConnectOptions, Database};
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
    let secret_hmac = Hmac::new_from_slice(config.secret_key.as_bytes()).unwrap();

    debug!("Config: {:?}", config);

    // let args = Args::parse();
    let db_conn_options = ConnectOptions::new(config.db.url.to_string());
    let db_connection = Database::connect(db_conn_options).await.unwrap();

    let sub_manager = Arc::new(SubscriptionManager::new(
        db_connection,
        secret_hmac.clone(),
        config.http.subscriber_url.clone(),
    ));

    let templater = Arc::new(TemplaterService::new("http://127.0.0.1:8000"));
    let credentials = Arc::new(SimpleCredentials::from_config(&config));
    let pipelines = Arc::new(SimplePipelineStorage::from_config(&config));
    let directory = Arc::new(MemoryRecipientDirectory::new(
        config.projects[0].recipients.clone(),
    ));

    let mut engine = Engine::new();

    engine.add_plugin(Arc::new(TelegramPlugin::new(
        templater.clone(),
        credentials.clone(),
    )));
    engine.add_plugin(Arc::new(EmailPlugin::new(templater, credentials)));
    engine.add_plugin(sub_manager.clone());

    let event_handler = EventHandler {
        pipeline_storage: pipelines.clone(),
        engine,
        recipient_storage: directory.clone(),
    }
    .start();

    tokio::spawn(http::start(
        event_handler.clone(),
        sub_manager.clone(),
        secret_hmac,
    ));

    tokio::signal::ctrl_c().await.unwrap();
}
