mod workflow;
mod plugin_registry;
mod plugin;
mod plugins;
mod message;
mod workflow_executor;

use clap::{Parser, Subcommand};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};
use std::fs;
use tracing;
use crate::workflow::{SerializedWorkflow, ParsedWorkflow};
use crate::plugin_registry::PluginRegistry;
use crate::plugins::noop::NoOpPlugin;
use crate::plugins::manual_trigger::ManualTriggerPlugin;
use std::sync::Arc;
use crate::workflow_executor::WorkflowExecutor;
use crate::message::Message;

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
            let Ok(contents) = fs::read_to_string(&file) else {
                tracing::error!("Failed to read file: {}", file);
                std::process::exit(1);
            };
            
            let json: SerializedWorkflow = serde_json::from_str(&contents).unwrap();
            let workflow = ParsedWorkflow::try_from(json).expect("Failed to parse workflow");

            let mut registry = PluginRegistry::new();
            registry.load_plugin(Arc::new(NoOpPlugin));
            registry.load_plugin(Arc::new(ManualTriggerPlugin));

            let executor = WorkflowExecutor::new(Arc::new(registry));
            tracing::info!("Successfully loaded JSON file from: {}", file);

            // Find trigger node to get its ID
            let Some(trigger_node) = executor.find_trigger_node(&workflow) else {
                tracing::error!("No trigger node found in workflow");
                std::process::exit(1);
            };

            // Create message with trigger node ID
            let message = Message::new(trigger_node.id);
            executor.execute_workflow(&workflow, message);
        }
    }
}
