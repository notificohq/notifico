pub mod env;
pub mod memory;

use crate::error::EngineError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum CredentialSelector {
    ByName(String),
}

/// Generic credential with type information.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Credential {
    pub transport: String,
    pub value: String,
}

impl FromStr for Credential {
    type Err = EngineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (transport, value) = s
            .split_once(':')
            .ok_or(EngineError::InvalidCredentialFormat)?;
        let (transport, value) = (transport.to_owned(), value.to_owned());
        Ok(Self { transport, value })
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
    async fn get_credential(
        &self,
        project: Uuid,
        selector: &CredentialSelector,
    ) -> Result<Credential, EngineError>;
}

impl dyn CredentialStorage {
    pub async fn resolve<T>(
        &self,
        project: Uuid,
        selector: CredentialSelector,
    ) -> Result<T, EngineError>
    where
        T: TypedCredential,
    {
        let credential = self.get_credential(project, &selector).await?;
        if credential.transport != T::TRANSPORT_NAME {
            return Err(EngineError::InvalidCredentialFormat);
        }
        credential.try_into()
    }
}
