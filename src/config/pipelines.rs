use crate::config::Config;
use notifico_core::error::EngineError;
use notifico_core::pipeline::{Pipeline, PipelineStorage};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

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
