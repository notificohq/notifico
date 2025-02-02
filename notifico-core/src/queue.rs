use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::error::Error;
use std::ops::Deref;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy)]
pub enum Outcome {
    Accepted,
    Rejected,
    Released,
}

pub enum MessageKind {
    Json,
    Object,
}

#[async_trait]
pub trait SenderChannel: Send + Sync {
    async fn send_object(
        &self,
        message: Box<dyn Any + Send + Sync + 'static>,
    ) -> Result<Outcome, Box<dyn std::error::Error>>;
    fn message_kind(&self) -> MessageKind;
}

impl dyn SenderChannel {
    pub async fn send(
        &self,
        object: impl Serialize + Send + Sync + 'static,
    ) -> Result<Outcome, Box<dyn Error>> {
        let boxed_object: Box<dyn Any + Send + Sync + 'static> = match self.message_kind() {
            MessageKind::Json => Box::new(serde_json::to_string(&object)?),
            MessageKind::Object => Box::new(object),
        };
        self.send_object(boxed_object).await
    }
}

#[async_trait]
pub trait ReceiverChannel: Send + Sync {
    async fn receive_object(
        &self,
    ) -> Result<
        (
            Box<dyn Any + Send + Sync + 'static>,
            oneshot::Sender<Outcome>,
        ),
        Box<dyn Error>,
    >;

    fn message_kind(&self) -> MessageKind;
}

impl dyn ReceiverChannel {
    pub async fn receive<T>(&self) -> Result<(T, oneshot::Sender<Outcome>), Box<dyn Error>>
    where
        T: for<'de> Deserialize<'de> + Send + Sync + Clone + Sized + 'static,
    {
        let result = self.receive_object().await?;
        match self.message_kind() {
            MessageKind::Json => {
                let message = result.0.downcast::<String>().unwrap();
                let message = serde_json::from_str(&message);
                match message {
                    Ok(message) => Ok((message, result.1)),
                    Err(err) => {
                        result.1.send(Outcome::Rejected).unwrap();
                        Err(Box::new(err))
                    }
                }
            }
            MessageKind::Object => {
                let message = result.0.downcast::<T>().unwrap();
                Ok((message.deref().clone(), result.1))
            }
        }
    }
}

#[async_trait]
impl SenderChannel for flume::Sender<Box<dyn Any + Send + Sync + 'static>> {
    async fn send_object(
        &self,
        message: Box<dyn Any + Send + Sync + 'static>,
    ) -> Result<Outcome, Box<dyn Error>> {
        self.send_async(message).await.map_err(Box::new)?;
        Ok(Outcome::Accepted)
    }

    fn message_kind(&self) -> MessageKind {
        MessageKind::Object
    }
}

#[async_trait]
impl ReceiverChannel for flume::Receiver<Box<dyn Any + Send + Sync + 'static>> {
    async fn receive_object(
        &self,
    ) -> Result<
        (
            Box<dyn Any + Send + Sync + 'static>,
            oneshot::Sender<Outcome>,
        ),
        Box<dyn Error>,
    > {
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            let _ = rx.await;
        });
        Ok((self.recv_async().await.map_err(Box::new)?, tx))
    }

    fn message_kind(&self) -> MessageKind {
        MessageKind::Object
    }
}
