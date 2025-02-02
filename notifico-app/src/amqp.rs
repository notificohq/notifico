use async_trait::async_trait;
use backoff::future::retry;
use backoff::ExponentialBackoff;
use fe2o3_amqp::connection::ConnectionHandle;
use fe2o3_amqp::link::delivery::DeliveryInfo;
use fe2o3_amqp::session::SessionHandle;
use fe2o3_amqp::types::definitions::AmqpError;
use fe2o3_amqp::types::definitions::Error as Fe2o3Error;
use fe2o3_amqp::types::messaging::Message;
use fe2o3_amqp::{Connection, Receiver, Sender, Session};
use notifico_core::queue::{MessageKind, Outcome, ReceiverChannel, SenderChannel};
use std::any::Any;
use std::error::Error;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use url::Url;

pub struct AmqpClient {
    #[allow(dead_code)]
    connection: ConnectionHandle<()>,
    session: SessionHandle<()>,
}

impl AmqpClient {
    pub async fn connect(url: Url, container_id: String) -> anyhow::Result<Self> {
        info!("Connecting to AMQP broker: {}", url);
        let mut connection = retry(ExponentialBackoff::default(), || async {
            Ok(Connection::open(container_id.clone(), url.clone()).await?)
        })
        .await?;
        info!("Connected to AMQP broker: {}", url);
        let session = Session::begin(&mut connection).await?;
        Ok(Self {
            connection,
            session,
        })
    }

    pub async fn create_sender(
        &mut self,
        address: &str,
        link_name: &str,
    ) -> anyhow::Result<AmqpSender> {
        Ok(AmqpSender {
            sender: Arc::new(Mutex::new(
                Sender::attach(&mut self.session, link_name, address).await?,
            )),
        })
    }

    pub async fn create_receiver(
        &mut self,
        address: &str,
        link_name: &str,
    ) -> anyhow::Result<AmqpReceiver> {
        let mut receiver = Receiver::attach(&mut self.session, link_name, address).await?;
        let (outcomes_tx, outcomes_rx) = flume::unbounded::<(Outcome, DeliveryInfo)>();
        let (message_tx, message_rx) = flume::unbounded();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(outcome) = outcomes_rx.recv_async() => {
                        match outcome.0{
                            Outcome::Accepted => {
                                receiver.accept(outcome.1).await.unwrap();
                            }
                            Outcome::Rejected => {
                                receiver.reject(outcome.1, Fe2o3Error::new(AmqpError::InternalError, None, None)).await.unwrap();
                            }
                            Outcome::Released => {
                                receiver.release(outcome.1).await.unwrap();
                            }
                        }
                    }
                    Ok(message) = receiver.recv::<String>() => {
                        message_tx.send_async(message.into_parts()).await.unwrap();
                    }
                    else => break
                }
            }
        });

        Ok(AmqpReceiver {
            message_receiver: message_rx,
            outcome_sender: outcomes_tx,
        })
    }

    pub async fn channel(
        &mut self,
        address: &str,
        link_name: &str,
    ) -> anyhow::Result<(AmqpSender, AmqpReceiver)> {
        Ok((
            self.create_sender(address, &format!("{link_name}-sender"))
                .await?,
            self.create_receiver(address, &format!("{link_name}-receiver"))
                .await?,
        ))
    }
}

#[derive(Clone)]
pub struct AmqpSender {
    sender: Arc<Mutex<Sender>>,
}

#[async_trait]
impl SenderChannel for AmqpSender {
    async fn send_object(
        &self,
        message: Box<dyn Any + Send + Sync + 'static>,
    ) -> Result<Outcome, Box<dyn Error>> {
        let mut sender_lk = self.sender.lock().await;
        let message = message.downcast::<String>().unwrap();
        sender_lk.send(message.deref().clone()).await?;
        Ok(Outcome::Accepted)
    }

    fn message_kind(&self) -> MessageKind {
        MessageKind::Json
    }
}

#[derive(Clone)]
pub struct AmqpReceiver {
    message_receiver: flume::Receiver<(DeliveryInfo, Message<String>)>,
    outcome_sender: flume::Sender<(Outcome, DeliveryInfo)>,
}

#[async_trait]
impl ReceiverChannel for AmqpReceiver {
    async fn receive_object(
        &self,
    ) -> Result<
        (
            Box<dyn Any + Send + Sync + 'static>,
            tokio::sync::oneshot::Sender<Outcome>,
        ),
        Box<dyn Error>,
    > {
        let (info, message) = self.message_receiver.recv_async().await?;

        let (tx, rx) = tokio::sync::oneshot::channel();
        let outcome_sender = self.outcome_sender.clone();

        tokio::spawn(async move {
            let outcome = rx.await.unwrap_or(Outcome::Released);
            outcome_sender.send_async((outcome, info)).await.unwrap();
        });

        Ok((Box::new(message.body), tx))
    }

    fn message_kind(&self) -> MessageKind {
        MessageKind::Json
    }
}
