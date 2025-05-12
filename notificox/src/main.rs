mod event_emitter;
mod message;
mod plugin;
mod plugin_registry;
mod plugins;
mod schemas;
mod workflow;

use crate::plugin_registry::PluginRegistry;
use crate::plugins::debug::DebugPlugin;
use crate::plugins::manual_trigger::ManualTriggerPlugin;
use crate::plugins::manual_trigger::ManualTriggerService;
use crate::plugins::noop::NoOpPlugin;
use crate::workflow::ParsedWorkflow;
use clap::{Parser, Subcommand};
use event_emitter::EventEmitter;
use schemas::SerializedWorkflow;
use std::fs;
use std::sync::Arc;
use tracing;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};
use uuid::Uuid;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run {
        #[arg(short, long)]
        file: String,
    },
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "notificox=info") };
    }

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run { file } => {
            let event_emitter = EventEmitter::new();
            let manual_trigger_service = Arc::new(ManualTriggerService::new(event_emitter.clone()));

            let mut registry = PluginRegistry::new();
            registry.load_plugin(Arc::new(NoOpPlugin));
            registry.load_plugin(Arc::new(ManualTriggerPlugin::new(
                manual_trigger_service.clone(),
            )));
            registry.load_plugin(Arc::new(DebugPlugin));

            let registry = Arc::new(registry);

            let Ok(contents) = fs::read_to_string(&file) else {
                tracing::error!("Failed to read file: {}", file);
                std::process::exit(1);
            };
            let json: SerializedWorkflow = serde_json::from_str(&contents).unwrap();
            let workflow = ParsedWorkflow::new(json);
            tracing::info!("Successfully loaded JSON file from: {}", file);

            let workflow_id = Uuid::now_v7();
            manual_trigger_service.trigger(workflow_id);

            let _ = tokio::signal::ctrl_c().await;
        }
    }
}
