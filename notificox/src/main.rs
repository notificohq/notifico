mod message;
mod plugin;
mod plugin_registry;
mod plugins;
mod workflow;
mod workflow_executor;

use crate::message::Message;
use crate::plugin_registry::PluginRegistry;
use crate::plugins::debug::DebugPlugin;
use crate::plugins::manual_trigger::ManualTriggerPlugin;
use crate::plugins::noop::NoOpPlugin;
use crate::workflow::{ParsedWorkflow, SerializedWorkflow};
use crate::workflow_executor::WorkflowExecutor;
use clap::{Parser, Subcommand};
use std::fs;
use std::sync::Arc;
use tracing;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

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
            let mut registry = PluginRegistry::new();
            registry.load_plugin(Arc::new(NoOpPlugin));
            registry.load_plugin(Arc::new(ManualTriggerPlugin));
            registry.load_plugin(Arc::new(DebugPlugin));

            let Ok(contents) = fs::read_to_string(&file) else {
                tracing::error!("Failed to read file: {}", file);
                std::process::exit(1);
            };

            let json: SerializedWorkflow = serde_json::from_str(&contents).unwrap();
            let workflow = ParsedWorkflow::try_from(json).expect("Failed to parse workflow");

            let executor = WorkflowExecutor::new(Arc::new(registry));
            tracing::info!("Successfully loaded JSON file from: {}", file);

            // Find all trigger nodes
            let trigger_nodes = executor.find_trigger_nodes(&workflow);
            if trigger_nodes.is_empty() {
                tracing::error!("No trigger nodes found in workflow");
                std::process::exit(1);
            }

            // Create and execute messages for each trigger node
            for trigger_node in trigger_nodes {
                tracing::info!("Executing workflow with trigger node: {}", trigger_node.id);
                let message = Message::new(trigger_node.id);
                executor.execute_workflow(&workflow, message).await;
            }
        }
    }
}
