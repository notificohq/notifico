use crate::credentials::{CredentialSelector, CredentialStorage, RawCredential};
use crate::error::EngineError;
use async_trait::async_trait;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Debug)]
struct CredentialKey {
    project: Uuid,
    name: String,
}

#[derive(Default, Debug)]
pub struct EnvCredentialStorage(HashMap<CredentialKey, RawCredential>);

#[derive(Serialize, ToSchema, Default, Debug)]
pub struct CredentialRestItem {
    id: Uuid,
    project_id: Uuid,
    name: String,
    r#type: String,
}

impl EnvCredentialStorage {
    pub fn new() -> Self {
        let mut storage = HashMap::new();

        let re = Regex::new("^NOTIFICO_CRED_(?:([[:xdigit:]]{8}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{12})_)?(.+)$").unwrap();
        for (name, value) in std::env::vars() {
            let Some(captures) = re.captures(&name) else {
                continue;
            };

            let project = captures
                .get(1)
                .map_or_else(Uuid::nil, |m| Uuid::parse_str(m.as_str()).unwrap());
            let name = captures.get(2).unwrap().as_str();
            let credential = RawCredential::from_str(&value).unwrap();

            storage.insert(
                CredentialKey {
                    project,
                    name: name.to_lowercase(),
                },
                credential,
            );
        }

        info!(
            "Imported {} credentials from environment variables",
            storage.len()
        );
        Self(storage)
    }

    pub fn list(&self) -> Vec<CredentialRestItem> {
        let mut items = Vec::with_capacity(self.0.len());
        for (key, credential) in &self.0 {
            items.push(CredentialRestItem {
                id: Uuid::now_v7(), // This id is purely synthetic, as refine requires ID in DataGrid
                project_id: key.project,
                name: key.name.clone(),
                r#type: credential.transport.clone(),
            });
        }
        items.sort_by(|item1, item2| natord::compare(&item1.name, &item2.name));
        items
    }
}

#[async_trait]
impl CredentialStorage for EnvCredentialStorage {
    async fn get_credential(
        &self,
        project: Uuid,
        selector: &CredentialSelector,
    ) -> Result<RawCredential, EngineError> {
        match selector {
            CredentialSelector::ByName(name) => {
                let key = CredentialKey {
                    project,
                    name: name.to_lowercase(),
                };

                self.0
                    .get(&key)
                    .cloned()
                    .ok_or(EngineError::CredentialNotFound)
            }
        }
    }
}
