use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use url::Url;
use uuid::Uuid;

#[derive(Debug)]
pub enum TemplaterError {
    RequestError(reqwest::Error),
    UrlError(url::ParseError),
}

impl From<reqwest::Error> for TemplaterError {
    fn from(err: reqwest::Error) -> Self {
        TemplaterError::RequestError(err)
    }
}

impl From<url::ParseError> for TemplaterError {
    fn from(err: url::ParseError) -> Self {
        TemplaterError::UrlError(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RenderResponse(pub Map<String, Value>);

#[derive(Debug, Serialize, Deserialize)]
struct RenderRequest {
    template_id: Uuid,
    context: Map<String, Value>,
}

#[async_trait]
pub trait Templater: Send + Sync {
    async fn render(
        &self,
        template_type: &str,
        template_id: Uuid,
        context: Map<String, Value>,
    ) -> Result<RenderResponse, TemplaterError>;
}

pub struct TemplaterService {
    client: Client,
    templater_baseurl: Url,
}

impl TemplaterService {
    pub fn new(templater_baseurl: &str) -> Self {
        TemplaterService {
            client: Client::builder().build().unwrap(),
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
