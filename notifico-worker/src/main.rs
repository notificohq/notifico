mod amqp;

use clap::Parser;
use figment::providers::Toml;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use notifico_core::config::credentials::MemoryCredentialStorage;
use notifico_core::config::pipelines::MemoryPipelineStorage;
use notifico_core::config::pipelines::PipelineConfig;
use notifico_core::config::Config;
use notifico_core::engine::Engine;
use notifico_core::pipeline::runner::PipelineRunner;
use notifico_smpp::SmppPlugin;
use notifico_smtp::EmailPlugin;
use notifico_telegram::TelegramPlugin;
use notifico_template::LocalTemplater;
use notifico_whatsapp::WaBusinessPlugin;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser)]
struct Args {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(|| "notifico.toml".into());

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let config: Config = Figment::new()
        .merge(Toml::file(config_path))
        .merge(Env::prefixed("NOTIFICO_"))
        .extract()
        .unwrap();

    info!("Config: {:#?}", config);

    let credential_config: serde_json::Value = Figment::new()
        .merge(Toml::file(config.credentials.clone()))
        .merge(Env::prefixed("NOTIFICO_CREDENTIAL_"))
        .extract()
        .unwrap();

    let pipelines_config: PipelineConfig = Figment::new()
        .merge(Yaml::file(config.pipelines.clone()))
        .extract()
        .unwrap();

    let credentials = Arc::new(MemoryCredentialStorage::from_config(credential_config).unwrap());
    let pipelines = Arc::new(MemoryPipelineStorage::from_config(&pipelines_config));

    // Create Engine with plugins
    let mut engine = Engine::new();
    engine.add_plugin(Arc::new(LocalTemplater::new(&config.templates)));

    engine.add_plugin(Arc::new(TelegramPlugin::new(credentials.clone())));
    engine.add_plugin(Arc::new(EmailPlugin::new(credentials.clone())));
    engine.add_plugin(Arc::new(WaBusinessPlugin::new(credentials.clone())));
    engine.add_plugin(Arc::new(SmppPlugin::new(credentials.clone())));

    // Create PipelineRunner, the core component of the Notifico system
    let runner = Arc::new(PipelineRunner::new(pipelines.clone(), engine));

    tokio::spawn(amqp::start(runner.clone(), config.amqp));

    tokio::signal::ctrl_c().await.unwrap();
}
