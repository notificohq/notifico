use crate::credentials::{Credential, CredentialSelector, CredentialStorage};
use crate::error::EngineError;
use async_trait::async_trait;
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
    pub fn add_credential(&mut self, project: Uuid, name: String, credential: Credential) {
        self.0.insert(
            CredentialKey {
                project,
                name: Cow::Owned(name),
            },
            credential,
        );
    }
}

#[async_trait]
impl CredentialStorage for MemoryCredentialStorage {
    async fn get_credential(
        &self,
        project: Uuid,
        selector: &CredentialSelector,
    ) -> Result<Credential, EngineError> {
        match selector {
            CredentialSelector::ByName(name) => {
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
    }
}
