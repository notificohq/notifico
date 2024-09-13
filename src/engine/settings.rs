use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

pub trait Settings {
    fn get_credentials(&self, plugin: &str, id: Uuid) -> Option<Value>;
}

#[derive(Default)]
pub struct HashMapSettings {
    settings: HashMap<(String, Uuid), Value>,
}

impl HashMapSettings {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_credentials(&mut self, plugin: &str, id: Uuid, credentials: Value) {
        self.settings.insert((plugin.into(), id), credentials);
    }
}

impl Settings for HashMapSettings {
    fn get_credentials(&self, plugin: &str, id: Uuid) -> Option<Value> {
        self.settings.get(&(plugin.into(), id)).cloned()
    }
}

// Some(json!({ "token": "7488126039:AAG9HCfywfyZHkYwB_bWuE6jeeDFTHuvFpM" }))
