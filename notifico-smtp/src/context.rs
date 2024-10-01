use lettre::message::Mailbox;
use serde::{Deserialize, Serialize};

pub const EMAIL_FROM: &str = "email.from";
pub const EMAIL_SUBJECT: &str = "email.subject";
pub const EMAIL_BODY_HTML: &str = "email.body_html";
pub const EMAIL_BODY_PLAINTEXT: &str = "email.body_plaintext";
// pub const EMAIL_LIST_UNSUBSCRIBE: &str = "email.list_unsubscribe";

#[derive(Serialize, Deserialize, Clone)]
pub struct Email {
    #[serde(rename = "email.from")]
    pub from: Mailbox,
    #[serde(rename = "email.subject")]
    pub subject: String,
    #[serde(rename = "email.body_html")]
    pub body_html: String,
    #[serde(rename = "email.body_plaintext")]
    pub body_plaintext: String,
    #[serde(rename = "email.list_unsubscribe")]
    pub list_unsubscribe: Option<String>,
}
