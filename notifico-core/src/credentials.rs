use crate::error::EngineError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum CredentialSelector {
    ByName(String),
}

/// Generic credential with type information.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Credential {
    Long { r#type: String, value: Value },
    Short(String),
}

impl Credential {
    pub fn transport(&self) -> &str {
        match self {
            Credential::Long { r#type, .. } => r#type,
            Credential::Short(url) => url.split("://").next().unwrap(),
        }
    }
}

/// Specific credential types should implement this trait.
pub trait TypedCredential:
    TryFrom<Credential, Error = EngineError> + Serialize + for<'de> Deserialize<'de>
{
    const TRANSPORT_NAME: &'static str;
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
                .and_then(|c| c.try_into()),
        }
    }
}
