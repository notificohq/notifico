use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
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
pub struct RenderedContext(pub HashMap<String, Value>);

#[derive(Debug, Serialize, Deserialize)]
struct RenderRequest {
    template_id: Uuid,
    context: HashMap<String, Value>,
}

pub struct Templater {
    client: Client,
    templater_baseurl: Url,
}

impl Templater {
    pub fn new(templater_baseurl: &str) -> Self {
        Templater {
            client: Client::builder().build().unwrap(),
            templater_baseurl: Url::parse(templater_baseurl).unwrap(),
        }
    }

    pub async fn render(
        &self,
        template_type: &str,
        template_id: Uuid,
        context: HashMap<String, Value>,
    ) -> Result<RenderedContext, TemplaterError> {
        let url = self
            .templater_baseurl
            .join(&format!("/template/{template_type}/render"))?;

        let render_request = RenderRequest {
            template_id,
            context,
        };

        let template = self.client.post(url).json(&render_request).send().await?;
        Ok(template.json::<RenderedContext>().await?)
    }
}
