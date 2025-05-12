use std::collections::HashMap;
use std::sync::{Arc, Weak};

use crate::event_emitter::EventEmitter;
use crate::message::Message;
use crate::plugin::{NodeType, Outcome, Plugin};
use crate::schemas::SerializedNode;
use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

pub struct ManualTriggerService {
    trigger_event_emitter: EventEmitter,
    registered_tokens: Mutex<HashMap<Uuid, u32>>,
}

impl ManualTriggerService {
    pub fn new(trigger_event_emitter: EventEmitter) -> Self {
        Self {
            trigger_event_emitter,
            registered_tokens: Mutex::new(HashMap::new()),
        }
    }

    pub fn trigger(&self, id: Uuid) {
        let token = self.registered_tokens.lock().unwrap().get(&id).copied();
        if let Some(token) = token {
            self.trigger_event_emitter.emit(token);
        }
    }

    pub fn register_trigger(&self, _node: &SerializedNode, token: u32, workflow_id: Uuid) {
        self.registered_tokens
            .lock()
            .unwrap()
            .insert(workflow_id, token);
    }

    pub fn unregister_trigger(&self, token: u32) {
        let mut tokens = self.registered_tokens.lock().unwrap();
        tokens.retain(|_, &mut t| t != token);
    }
}

pub struct ManualTriggerPlugin {
    service: Weak<ManualTriggerService>,
}

impl ManualTriggerPlugin {
    pub fn new(service: Arc<ManualTriggerService>) -> Self {
        Self {
            service: Arc::downgrade(&service),
        }
    }
}

#[async_trait]
impl Plugin for ManualTriggerPlugin {
    async fn process_message(
        &self,
        _node: &SerializedNode,
        message: Message,
        _slot: Option<String>,
    ) -> Outcome {
        // Manual triggers don't need any execution logic
        Outcome::Return {
            message,
            slot: None,
        }
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.trigger.manual.v1".to_string(),
            is_trigger: true,
        }]
    }

    fn register_trigger(&self, node: &SerializedNode, token: u32, workflow_id: Uuid) {
        if let Some(service) = self.service.upgrade() {
            service.register_trigger(node, token, workflow_id);
        }
    }

    fn unregister_trigger(&self, token: u32) {
        if let Some(service) = self.service.upgrade() {
            service.unregister_trigger(token);
        }
    }
}
