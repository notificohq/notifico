use notifico_core::error::EngineError;
use notifico_core::pipeline::storage::PipelineStorage;
use notifico_core::pipeline::Pipeline;
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct PipelineConfig {
    pipelines: Vec<Pipeline>,
}

#[derive(Eq, PartialEq, Hash, Clone)]
struct PipelineKey<'a> {
    project: Uuid,
    event: Cow<'a, str>,
}

#[derive(Default)]
pub struct MemoryPipelineStorage(HashMap<PipelineKey<'static>, Vec<Arc<Pipeline>>>);

impl MemoryPipelineStorage {
    pub fn from_config(config: &PipelineConfig) -> Self {
        let mut slf = Self::default();
        for pipeline in config.pipelines.iter() {
            let pipeline = Arc::new(pipeline.clone());
            for event in pipeline.events.iter() {
                let key = PipelineKey {
                    project: Uuid::nil(),
                    event: Cow::Owned(event.clone()),
                };
                slf.0
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(pipeline.clone())
            }
        }
        slf
    }
}

impl PipelineStorage for MemoryPipelineStorage {
    fn get_pipelines(&self, project: Uuid, event_name: &str) -> Result<Vec<Pipeline>, EngineError> {
        if let Some(pipelines) = self.0.get(&PipelineKey {
            project,
            event: event_name.into(),
        }) {
            Ok(pipelines.iter().map(|p| p.as_ref()).cloned().collect())
        } else {
            Ok(Vec::new())
        }
    }
}
