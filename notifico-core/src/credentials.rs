use crate::error::EngineError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum CredentialSelector {
    ByName(String),
    Inline(Credential),
}

/// Generic credential with type information.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Credential {
    pub r#type: String,
    pub value: Value,
}

impl Credential {
    pub fn into_typed<T>(self) -> Result<T, EngineError>
    where
        T: TypedCredential,
    {
        if self.r#type != T::CREDENTIAL_TYPE {
            return Err(EngineError::InvalidCredentialFormat);
        }
        serde_json::from_value(self.value).map_err(|_| EngineError::InvalidCredentialFormat)
    }
}

/// Specific credential types should implement this trait.
pub trait TypedCredential: for<'de> Deserialize<'de> {
    const CREDENTIAL_TYPE: &'static str;
}

#[async_trait]
pub trait CredentialStorage: Send + Sync {
    async fn get_credential(&self, project: Uuid, name: &str) -> Result<Credential, EngineError>;
}

impl dyn CredentialStorage {
    pub async fn resolve<T>(
        &self,
        project: Uuid,
        name: CredentialSelector,
    ) -> Result<T, EngineError>
    where
        T: TypedCredential,
    {
        match name {
            CredentialSelector::ByName(name) => self
                .get_credential(project, &name)
                .await
                .and_then(|c| c.into_typed()),
            CredentialSelector::Inline(credential) => credential.into_typed(),
        }
    }
}

pub struct DummyCredentialStorage;

#[async_trait]
impl CredentialStorage for DummyCredentialStorage {
    async fn get_credential(&self, _project: Uuid, _name: &str) -> Result<Credential, EngineError> {
        Err(EngineError::CredentialNotFound)
    }
}
