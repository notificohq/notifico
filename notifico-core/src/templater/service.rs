use crate::templater::{RenderResponse, Templater, TemplaterError};
use async_trait::async_trait;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct RenderRequest {
    template_id: Uuid,
    context: Map<String, Value>,
}

pub struct TemplaterService {
    client: ClientWithMiddleware,
    templater_baseurl: Url,
}

impl TemplaterService {
    pub fn new(templater_baseurl: &str) -> Self {
        TemplaterService {
            client: ClientBuilder::new(Client::builder().build().unwrap())
                .with(TracingMiddleware::default())
                .build(),
            templater_baseurl: Url::parse(templater_baseurl).unwrap(),
        }
    }
}

#[async_trait]
impl Templater for TemplaterService {
    async fn render(
        &self,
        template_type: &str,
        template_id: Uuid,
        context: Map<String, Value>,
    ) -> Result<RenderResponse, TemplaterError> {
        let url = self
            .templater_baseurl
            .join(&format!("/template/{template_type}/render"))?;

        let render_request = RenderRequest {
            template_id,
            context,
        };

        let template = self.client.post(url).json(&render_request).send().await?;
        Ok(template.json::<RenderResponse>().await?)
    }
}
