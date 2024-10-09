use async_trait::async_trait;
use notifico_core::{
    engine::PipelineContext,
    engine::{EnginePlugin, StepOutput},
    error::EngineError,
    pipeline::SerializedStep,
};
use reqwest_middleware::reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use url::Url;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "templater.load")]
    Load { templates: Vec<Uuid> },
}

pub struct TemplaterService {
    client: ClientWithMiddleware,
    templater_baseurl: Url,
}

impl TemplaterService {
    pub fn new(templater_baseurl: &str) -> Self {
        Self {
            client: ClientBuilder::new(Client::builder().build().unwrap())
                .with(TracingMiddleware::default())
                .build(),
            templater_baseurl: Url::parse(templater_baseurl).unwrap(),
        }
    }
}

#[async_trait]
impl EnginePlugin for TemplaterService {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::Load { templates } => {
                if context.recipient.is_none() {
                    return Err(EngineError::RecipientNotSet);
                };

                for template_id in templates {
                    let template_type = context.channel.clone();
                    let url = self
                        .templater_baseurl
                        .join(&format!("/template/{template_type}/render"))
                        .unwrap();

                    let render_request = RenderRequest {
                        template_id,
                        context: context.event_context.0.clone(),
                    };

                    let template = self
                        .client
                        .post(url)
                        .json(&render_request)
                        .send()
                        .await
                        .unwrap();
                    let rendered_template = template.json().await.unwrap();

                    context.messages.push(rendered_template)
                }

                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        vec!["templater.load".into()]
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct RenderRequest {
    template_id: Uuid,
    context: Map<String, Value>,
}
