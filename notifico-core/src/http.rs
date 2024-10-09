use uuid::Uuid;

#[derive(Clone)]
pub struct AuthorizedRecipient {
    pub project_id: Uuid,
    pub recipient_id: Uuid,
}
