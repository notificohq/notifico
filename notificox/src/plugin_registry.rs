use crate::plugin::Plugin;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct PluginRegistry {
    pub plugins: Vec<Arc<dyn Plugin>>,
    pub nodes: HashMap<String, Arc<dyn Plugin>>,
    pub triggers: HashSet<String>,
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
}
