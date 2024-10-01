use notifico_core::credentials::{Credential, Credentials};
use notifico_core::error::EngineError;
use notifico_core::pipeline::{Pipeline, PipelineStorage};
use notifico_core::recipient::Recipient;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub projects: Vec<Project>,
    pub secret_key: String,
    pub http: Http,
    pub db: Database,
}

#[derive(Debug, Deserialize)]
pub struct Http {
    pub bind: String,
    pub subscriber_url: Url,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub url: Url,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(default = "Uuid::nil")]
    pub id: Uuid,
    pub pipelines: Vec<Pipeline>,
    pub credentials: Vec<Credential>,
    pub recipients: Vec<Recipient>,
}

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

    fn add_project(&mut self, project: Uuid, pipelines: Vec<Credential>) {
        if self.creds.get(&project).is_some() {
            panic!("Project already exists: {}", project);
        }

        self.creds.insert(project, pipelines);
    }
}

impl Credentials for SimpleCredentials {
    fn get_credential(
        &self,
        project: Uuid,
        r#type: &str,
        name: &str,
    ) -> Result<Value, EngineError> {
        let Some(creds) = self.creds.get(&project) else {
            return Err(EngineError::ProjectNotFound(project));
        };

        for cred in creds.iter() {
            if cred.r#type == r#type && cred.name == name {
                return Ok(cred.value.clone());
            }
        }
        Err(EngineError::CredentialNotFound(
            r#type.to_string().into(),
            name.into(),
        ))
    }
}

#[derive(Default)]
pub struct SimplePipelineStorage {
    eventmap: HashMap<Uuid, HashMap<String, Vec<Arc<Pipeline>>>>,
}

impl SimplePipelineStorage {
    pub fn from_config(config: &Config) -> Self {
        let mut slf = Self::default();
        for project in config.projects.iter() {
            slf.add_project(project.id, project.pipelines.clone());
        }
        slf
    }

    pub fn add_project(&mut self, project: Uuid, pipelines: Vec<Pipeline>) {
        if self.eventmap.get(&project).is_some() {
            panic!("Project already exists: {}", project);
        }

        let mut eventmap = HashMap::new();

        for pipeline in pipelines.into_iter().map(|p| Arc::new(p)) {
            for event in pipeline.events.iter().cloned() {
                eventmap
                    .entry(event)
                    .or_insert_with(Vec::new)
                    .push(pipeline.clone());
            }
        }

        self.eventmap.insert(project, eventmap);
    }
}

impl PipelineStorage for SimplePipelineStorage {
    fn get_pipelines(&self, project: Uuid, event_name: &str) -> Result<Vec<Pipeline>, EngineError> {
        let Some(eventmap) = self.eventmap.get(&project) else {
            return Err(EngineError::ProjectNotFound(project));
        };

        if let Some(pipelines) = eventmap.get(event_name) {
            Ok(pipelines.iter().map(|p| p.as_ref().clone()).collect())
        } else {
            Ok(vec![])
        }
    }
}
