use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
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

#[async_trait]
pub trait Templater: Send + Sync {
    async fn render(
        &self,
        template_type: &str,
        template_id: Uuid,
        context: Map<String, Value>,
    ) -> Result<RenderResponse, TemplaterError>;
}
