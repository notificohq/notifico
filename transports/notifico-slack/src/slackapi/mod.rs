mod schemas;

use crate::slackapi::schemas::{
    CompleteUploadExternalRequest, CompleteUploadExternalRequestFile, GetUploadUrlExternalRequest,
    GetUploadUrlExternalResponse,
};
use reqwest::header::AUTHORIZATION;
use reqwest::Body;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Deserialize, Debug)]
pub struct SlackStatusResponse {
    ok: bool,
    error: Option<String>,
}

impl SlackStatusResponse {
    fn into_result(self) -> Result<Self, SlackError> {
        match self.ok {
            true => Ok(self),
            false => Err(SlackError::ApiError {
                error: self.error.unwrap_or_default(),
            }),
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
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
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

    pub async fn upload_file(
        &self,
        token: &str,
        file: File,
        filename: &str,
        length: u64,
        channel: &str,
    ) -> Result<(), SlackError> {
        let req = GetUploadUrlExternalRequest {
            filename: filename.to_string(),
            length,
        };

        let req_url = "https://slack.com/api/files.getUploadURLExternal?".to_string()
            + &serde_urlencoded::to_string(req).unwrap();

        let resp = self
            .client
            .get(req_url)
            .header(AUTHORIZATION, String::from("Bearer ") + token)
            .send()
            .await?;

        let resp = resp.json::<GetUploadUrlExternalResponse>().await?;
        let file_id = resp.file_id;

        let stream = FramedRead::new(file, BytesCodec::new());

        let _resp = self
            .client
            .post(resp.upload_url)
            .header(AUTHORIZATION, String::from("Bearer ") + token)
            .body(Body::wrap_stream(stream))
            .send()
            .await?;

        let req = CompleteUploadExternalRequest {
            files: vec![CompleteUploadExternalRequestFile { id: file_id }],
            channel_id: Some(channel.to_string()),
        };

        let resp = self
            .client
            .post("https://slack.com/api/files.completeUploadExternal")
            .header(AUTHORIZATION, String::from("Bearer ") + token)
            .json(&req)
            .send()
            .await?;

        resp.json::<SlackStatusResponse>().await?.into_result()?;

        Ok(())
    }
}
