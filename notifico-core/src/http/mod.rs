pub mod auth;

use hmac::Hmac;
use sha2::Sha256;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthorizedRecipient {
    pub project_id: Uuid,
    pub recipient_id: Uuid,
}

#[derive(Clone)]
pub struct SecretKey(pub Hmac<Sha256>);
