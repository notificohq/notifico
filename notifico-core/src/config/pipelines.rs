use crate::error::EngineError;
use crate::pipeline::storage::PipelineStorage;
use crate::pipeline::Pipeline;
use crate::step::SerializedStep;
use async_trait::async_trait;
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize, Clone)]
struct PipelineItem {
    pub events: Vec<String>,
    #[serde(default = "Uuid::nil")]
    pub project: Uuid,
    pub channel: String,
    pub steps: Vec<SerializedStep>,
}

impl From<PipelineItem> for Pipeline {
    fn from(value: PipelineItem) -> Self {
        Self {
            id: Uuid::nil(),
            project_id: value.project,
            channel: value.channel,
            steps: value.steps,
        }
    }
}

#[derive(Deserialize)]
pub struct PipelineConfig {
    pipelines: Vec<PipelineItem>,
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
        for pipeline_item in config.pipelines.iter() {
            let pipeline = Arc::new(Pipeline::from(pipeline_item.clone()));

            for event in pipeline_item.events.iter() {
                let pipeline = pipeline.clone();
                let key = PipelineKey {
                    project: Uuid::nil(),
                    event: Cow::Owned(event.clone()),
                };

                slf.0.entry(key).or_insert_with(Vec::new).push(pipeline)
            }
        }
        slf
    }
}

#[async_trait]
impl PipelineStorage for MemoryPipelineStorage {
    async fn get_pipelines_for_event(
        &self,
        project: Uuid,
        event_name: &str,
    ) -> Result<Vec<Pipeline>, EngineError> {
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
