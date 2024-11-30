use async_trait::async_trait;
use fe2o3_amqp::connection::ConnectionHandle;
use fe2o3_amqp::session::SessionHandle;
use fe2o3_amqp::{Connection, Receiver, Sender, Session};
use notifico_core::pipeline::event::EventHandler;
use notifico_core::queue::{ReceiverChannel, SenderChannel};
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use url::Url;
use uuid::Uuid;

pub struct AmqpClient {
    connection: ConnectionHandle<()>,
    session: SessionHandle<()>,
}

impl AmqpClient {
    pub async fn connect(url: Url, container_id: String) -> anyhow::Result<Self> {
        info!("Connecting to AMQP broker: {}", url);
        let mut connection = Connection::open(container_id, url.clone()).await?;
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
        Ok(AmqpReceiver {
            receiver: Arc::new(Mutex::new(
                Receiver::attach(&mut self.session, link_name, address).await?,
            )),
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
    async fn send(&self, message: String) -> Result<(), Box<dyn Error>> {
        let mut sender_lk = self.sender.lock().await;
        sender_lk.send(message).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct AmqpReceiver {
    receiver: Arc<Mutex<Receiver>>,
}

#[async_trait]
impl ReceiverChannel for AmqpReceiver {
    async fn receive(&self) -> Result<String, Box<dyn Error>> {
        let mut receiver_lk = self.receiver.lock().await;
        let message = receiver_lk.recv::<String>().await?;
        receiver_lk.accept(&message).await?;
        Ok(message.body().clone())
    }
}
