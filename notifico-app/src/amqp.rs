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
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, watch, Mutex};
use tokio::{task, time};
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
        let (message_tx, message_rx) = flume::bounded(200);
        let (drop_tx, mut drop_rx) = watch::channel(false);

        let (accepted_tx, accepted_rx) = flume::unbounded::<DeliveryInfo>();
        let (rejected_tx, rejected_rx) = flume::unbounded::<DeliveryInfo>();
        let (released_tx, released_rx) = flume::unbounded::<DeliveryInfo>();

        let ack_buffer_len = 100;
        let ack_interval = Duration::from_secs(1);

        let loop_handle = tokio::spawn(async move {
            let mut interval = time::interval(ack_interval);

            let mut accepted_buffer: Vec<DeliveryInfo> = Vec::with_capacity(ack_buffer_len);

            loop {
                tokio::select! {
                    Ok(_) = drop_rx.changed() => {
                        receiver.detach().await.unwrap();
                        break;
                    },
                    Ok(outcome) = accepted_rx.recv_async()  => {
                        accepted_buffer.push(outcome);
                        if accepted_buffer.len() >= ack_buffer_len {
                            let drained: Vec<DeliveryInfo> = mem::replace(&mut accepted_buffer, Vec::with_capacity(ack_buffer_len));
                            receiver.accept_all(drained).await.unwrap();
                        }
                    }
                    _ = interval.tick() => {
                        let drained: Vec<DeliveryInfo> = mem::replace(&mut accepted_buffer, Vec::with_capacity(ack_buffer_len));
                        receiver.accept_all(drained).await.unwrap();
                    }
                    Ok(outcome) = rejected_rx.recv_async() => {
                        receiver.reject(outcome, Fe2o3Error::new(AmqpError::InternalError, None, None)).await.unwrap();
                    }
                    Ok(outcome) = released_rx.recv_async() => {
                        receiver.release(outcome).await.unwrap();
                    }
                    Ok(message) = receiver.recv::<String>(), if !message_tx.is_full() => {
                        let _ = message_tx.send_async(message.into_parts()).await;
                    }
                    else => {break;}
                }
            }
        });

        Ok(AmqpReceiver {
            message_receiver: message_rx,
            accepted_sender: accepted_tx,
            rejected_sender: rejected_tx,
            released_sender: released_tx,
            drop_sender: drop_tx,
            loop_handle: Some(loop_handle),
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
        sender_lk.send(*message).await?;
        Ok(Outcome::Accepted)
    }

    fn message_kind(&self) -> MessageKind {
        MessageKind::Json
    }
}

pub struct AmqpReceiver {
    message_receiver: flume::Receiver<(DeliveryInfo, Message<String>)>,
    accepted_sender: flume::Sender<DeliveryInfo>,
    rejected_sender: flume::Sender<DeliveryInfo>,
    released_sender: flume::Sender<DeliveryInfo>,
    drop_sender: watch::Sender<bool>,
    loop_handle: Option<task::JoinHandle<()>>,
}

#[async_trait]
impl ReceiverChannel for AmqpReceiver {
    async fn receive_object(
        &self,
    ) -> Result<
        (
            Box<dyn Any + Send + Sync + 'static>,
            oneshot::Sender<Outcome>,
        ),
        Box<dyn Error>,
    > {
        let (info, message) = self.message_receiver.recv_async().await?;

        let (tx, rx) = oneshot::channel();
        let accepted_sender = self.accepted_sender.clone();
        let rejected_sender = self.rejected_sender.clone();
        let released_sender = self.released_sender.clone();

        tokio::spawn(async move {
            let outcome = rx.await.unwrap_or(Outcome::Released);
            let channel = match outcome {
                Outcome::Accepted => accepted_sender,
                Outcome::Rejected => rejected_sender,
                Outcome::Released => released_sender,
            };
            channel.send_async(info).await.unwrap();
        });

        Ok((Box::new(message.body), tx))
    }

    fn message_kind(&self) -> MessageKind {
        MessageKind::Json
    }
}

impl Drop for AmqpReceiver {
    fn drop(&mut self) {
        let _ = self.drop_sender.send(true);
        futures::executor::block_on(self.loop_handle.take().unwrap()).unwrap();
    }
}
