use async_trait::async_trait;
use notifico_core::credentials::{Credential, CredentialStorage};
use notifico_core::error::EngineError;
use std::borrow::Cow;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Debug)]
struct CredentialKey<'a> {
    project: Uuid,
    name: Cow<'a, str>,
}

#[derive(Default, Debug)]
pub struct MemoryCredentialStorage(HashMap<CredentialKey<'static>, Credential>);

impl MemoryCredentialStorage {
    pub fn from_config(config: serde_json::Value) -> Result<Self, serde_json::Error> {
        let mut creds = MemoryCredentialStorage::default();

        let obj = config.as_object().unwrap().clone();
        for (r#type, v) in obj {
            let obj = v.as_object().unwrap().clone();
            for (name_or_project_id, value) in obj {
                if let Ok(project_id) = Uuid::parse_str(&name_or_project_id) {
                    for (name, value) in value.as_object().unwrap().iter() {
                        creds.add_credential(
                            project_id,
                            name.clone(),
                            r#type.clone(),
                            value.clone(),
                        );
                    }
                } else {
                    creds.add_credential(Uuid::nil(), name_or_project_id, r#type.clone(), value);
                };
            }
        }

        Ok(creds)
    }

    pub fn add_credential(
        &mut self,
        project: Uuid,
        name: String,
        r#type: String,
        value: serde_json::Value,
    ) {
        self.0.insert(
            CredentialKey {
                project,
                name: Cow::Owned(name),
            },
            Credential { r#type, value },
        );
    }
}

#[async_trait]
impl CredentialStorage for MemoryCredentialStorage {
    async fn get_credential(&self, project: Uuid, name: &str) -> Result<Credential, EngineError> {
        let key = CredentialKey {
            project,
            name: Cow::from(name),
        };

        self.0
            .get(&key)
            .cloned()
            .ok_or(EngineError::CredentialNotFound)
    }
}
