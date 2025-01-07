pub mod admin;
pub mod auth;

#[derive(Clone)]
pub struct SecretKey(pub Vec<u8>);
