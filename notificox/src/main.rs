mod message;
mod plugin;
mod plugin_registry;
mod plugins;
mod trigger_event_emitter;
mod trigger_service;
mod workflow;
mod workflow_executor;

use crate::plugin_registry::PluginRegistry;
use crate::plugins::debug::DebugPlugin;
use crate::plugins::manual_trigger::ManualTriggerPlugin;
use crate::plugins::manual_trigger::ManualTriggerService;
use crate::plugins::noop::NoOpPlugin;
use crate::trigger_service::TriggerService;
use crate::workflow::{ParsedWorkflow, SerializedWorkflow};
use crate::workflow_executor::WorkflowExecutor;
use clap::{Parser, Subcommand};
use std::fs;
use std::sync::Arc;
use tracing;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};
use trigger_event_emitter::TriggerEventEmitter;
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
            let trigger_event_emitter = TriggerEventEmitter::new();
            let manual_trigger_service =
                Arc::new(ManualTriggerService::new(trigger_event_emitter.clone()));

            let mut registry = PluginRegistry::new();
            registry.load_plugin(Arc::new(NoOpPlugin));
            registry.load_plugin(Arc::new(ManualTriggerPlugin::new(
                manual_trigger_service.clone(),
            )));
            registry.load_plugin(Arc::new(DebugPlugin));

            let registry = Arc::new(registry);
            let executor = Arc::new(WorkflowExecutor::new(registry.clone()));
            let trigger_service = Arc::new(TriggerService::new(registry.clone(), executor.clone()));

            let trigger_service_clone = trigger_service.clone();
            tokio::spawn(async move {
                let mut receiver = trigger_event_emitter.subscribe();
                while let Ok(token) = receiver.recv().await {
                    tracing::info!("Received trigger token: {}", token);
                    trigger_service_clone.trigger(token).await;
                }
            });

            let Ok(contents) = fs::read_to_string(&file) else {
                tracing::error!("Failed to read file: {}", file);
                std::process::exit(1);
            };
            let json: SerializedWorkflow = serde_json::from_str(&contents).unwrap();
            let workflow = ParsedWorkflow::try_from(json).expect("Failed to parse workflow");
            tracing::info!("Successfully loaded JSON file from: {}", file);

            let workflow_id = Uuid::now_v7();
            trigger_service
                .register_workflow(&workflow, workflow_id)
                .await;
            manual_trigger_service.trigger(workflow_id);

            let _ = tokio::signal::ctrl_c().await;
        }
    }
}
