use async_trait::async_trait;

#[async_trait]
pub trait SenderChannel: Send + Sync {
    async fn send(&self, message: String) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait ReceiverChannel: Send + Sync {
    async fn receive(&self) -> Result<String, Box<dyn std::error::Error>>;
}

// Example implementation using flume, useful for test purposes
#[async_trait]
impl SenderChannel for flume::Sender<String> {
    async fn send(&self, message: String) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.send_async(message).await.map_err(Box::new)?)
    }
}

#[async_trait]
impl ReceiverChannel for flume::Receiver<String> {
    async fn receive(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.recv_async().await.map_err(Box::new)?)
    }
}
