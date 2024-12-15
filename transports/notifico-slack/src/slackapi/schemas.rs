use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize)]
pub struct GetUploadUrlExternalRequest {
    pub filename: String,
    pub length: u64,
}

#[derive(Deserialize)]
pub struct GetUploadUrlExternalResponse {
    pub upload_url: Url,
    pub file_id: String,
}

#[derive(Serialize)]
pub struct CompleteUploadExternalRequestFile {
    pub id: String,
}

#[derive(Serialize)]
pub struct CompleteUploadExternalRequest {
    pub files: Vec<CompleteUploadExternalRequestFile>,
    pub channel_id: Option<String>,
}
