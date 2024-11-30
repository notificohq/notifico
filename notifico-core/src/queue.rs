use async_trait::async_trait;

#[async_trait]
pub trait SenderChannel {
    async fn send(&self, message: String) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait ReceiverChannel {
    async fn receive(&self) -> Result<String, Box<dyn std::error::Error>>;
}
