use crate::credentials::{Credential, CredentialSelector, CredentialStorage};
use crate::error::EngineError;
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

#[derive(Eq, PartialEq, Hash, Debug)]
struct CredentialKey {
    project: Uuid,
    name: String,
}

#[derive(Default, Debug)]
pub struct EnvCredentialStorage(HashMap<CredentialKey, Credential>);

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
            let credential = Credential::Short(value);

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
}

#[async_trait]
impl CredentialStorage for EnvCredentialStorage {
    async fn get_credential(
        &self,
        project: Uuid,
        selector: &CredentialSelector,
    ) -> Result<Credential, EngineError> {
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
