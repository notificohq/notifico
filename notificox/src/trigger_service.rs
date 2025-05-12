use crate::message::Message;
use crate::plugin_registry::PluginRegistry;
use crate::workflow::NodeId;
use crate::workflow::ParsedWorkflow;
use crate::workflow_executor::WorkflowExecutor;
use slab::Slab;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing;
use uuid::Uuid;

pub struct TriggerService {
    registry: Arc<PluginRegistry>,
    tokens: Mutex<Slab<(Uuid, NodeId)>>,
    workflows: Mutex<HashMap<Uuid, ParsedWorkflow>>,
    workflow_executor: Arc<WorkflowExecutor>,
}

impl TriggerService {
    pub fn new(registry: Arc<PluginRegistry>, workflow_executor: Arc<WorkflowExecutor>) -> Self {
        Self {
            registry,
            tokens: Mutex::new(Slab::new()),
            workflows: Mutex::new(HashMap::new()),
            workflow_executor,
        }
    }

    pub async fn register_workflow(&self, workflow: &ParsedWorkflow, workflow_id: Uuid) -> Uuid {
        tracing::info!("Registering workflow with ID: {}", workflow_id);

        // Find all trigger nodes
        let trigger_nodes = workflow.find_trigger_nodes(&self.registry);
        if trigger_nodes.is_empty() {
            tracing::error!("No trigger nodes found in workflow");
            std::process::exit(1);
        }

        let mut workflows = self.workflows.lock().await;
        workflows.insert(workflow_id, workflow.clone());

        // Register trigger nodes with their respective plugins
        for trigger_node in &trigger_nodes {
            if let Some(plugin) = self.registry.get_plugin(&trigger_node.r#type) {
                let mut tokens = self.tokens.lock().await;
                let token: u32 = tokens.insert((workflow_id, trigger_node.id)) as _;

                plugin.register_trigger(trigger_node, token, workflow_id);
                tracing::info!(
                    "Registered trigger node {} with plugin {} (token: {}, workflow_id: {})",
                    trigger_node.id,
                    trigger_node.r#type,
                    token,
                    workflow_id
                );
            } else {
                tracing::error!(
                    "Plugin {} not found for trigger node {}",
                    trigger_node.r#type,
                    trigger_node.id
                );
                std::process::exit(1);
            }
        }

        workflow_id
    }

    pub async fn trigger(&self, token: u32) {
        let workflows = self.workflows.lock().await;
        let (workflow_id, trigger_node_id) =
            self.tokens.lock().await.get(token as _).cloned().unwrap();

        tracing::info!(
            "Triggering workflow (token: {}, workflow_id: {})",
            token,
            workflow_id
        );

        let workflow = workflows.get(&workflow_id).cloned().unwrap();

        let message = Message::default();

        self.workflow_executor
            .execute_workflow(&workflow, message, trigger_node_id)
            .await;
    }
}
