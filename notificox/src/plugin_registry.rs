use crate::plugin::Plugin;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct PluginRegistry {
    plugins: Vec<Arc<dyn Plugin>>,
    nodes: HashMap<String, Arc<dyn Plugin>>,
    triggers: HashSet<String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            nodes: HashMap::new(),
            triggers: HashSet::new(),
        }
    }

    pub fn load_plugin(&mut self, plugin: Arc<dyn Plugin>) {
        // Add to plugins list
        self.plugins.push(plugin.clone());

        // Add to nodes mapping and triggers set
        for node_type in plugin.all_node_types() {
            self.nodes.insert(node_type.name.clone(), plugin.clone());
            if node_type.is_trigger {
                self.triggers.insert(node_type.name);
            }
        }
    }

    pub fn get_plugin(&self, node_type: &str) -> Option<&Arc<dyn Plugin>> {
        self.nodes.get(node_type)
    }

    pub fn is_trigger(&self, node_type: &str) -> bool {
        self.triggers.contains(node_type)
    }
}
