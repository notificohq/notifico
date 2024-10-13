use crate::config::Config;
use async_trait::async_trait;
use notifico_core::credentials::{Credential, Credentials};
use notifico_core::error::EngineError;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Default)]
pub struct SimpleCredentials {
    creds: HashMap<Uuid, Vec<Credential>>,
}

impl SimpleCredentials {
    pub fn from_config(config: &Config) -> Self {
        let mut slf = Self::default();
        for project in config.projects.iter() {
            slf.add_project(project.id, project.credentials.clone());
        }
        slf
    }

    fn add_project(&mut self, project: Uuid, credentials: Vec<Credential>) {
        if self.creds.insert(project, credentials).is_some() {
            panic!("Project already exists: {}", project);
        }
    }
}

#[async_trait]
impl Credentials for SimpleCredentials {
    async fn get_credential(
        &self,
        project: Uuid,
        r#type: &str,
        name: &str,
    ) -> Result<Credential, EngineError> {
        let Some(creds) = self.creds.get(&project) else {
            return Err(EngineError::ProjectNotFound(project));
        };

        for cred in creds.iter() {
            if cred.r#type == r#type && cred.name == name {
                return Ok(cred.clone());
            }
        }
        Err(EngineError::CredentialNotFound(
            r#type.to_string().into(),
            name.into(),
        ))
    }
}
