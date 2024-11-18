use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Deserialize, Debug)]
pub struct SlackStatusResponse {
    ok: bool,
    error: Option<String>,
}

impl SlackStatusResponse {
    fn into_result(self) -> Result<Self, SlackError> {
        if !self.ok {
            Err(SlackError::ApiError {
                error: self.error.unwrap_or_default(),
            })
        } else {
            Ok(self)
        }
    }
}

#[derive(Error, Debug)]
pub enum SlackError {
    #[error("{0}")]
    Request(#[from] reqwest::Error),
    #[error("Slack API returned error: {error:?}")]
    ApiError { error: String },
}

pub struct SlackApi {
    client: reqwest::Client,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum SlackMessage {
    Text { channel: String, text: String },
}

impl SlackApi {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn chat_post_message(
        &self,
        token: &str,
        message: SlackMessage,
    ) -> Result<SlackStatusResponse, SlackError> {
        let resp = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .header(AUTHORIZATION, String::from("Bearer ") + token)
            .json(&message)
            .send()
            .await?;

        resp.json::<SlackStatusResponse>().await?.into_result()
    }
}
