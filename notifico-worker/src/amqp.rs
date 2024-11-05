use crate::Amqp;
use fe2o3_amqp::acceptor::{ConnectionAcceptor, LinkAcceptor, LinkEndpoint, SessionAcceptor};
use fe2o3_amqp::{Connection, Receiver, Session};
use notifico_core::pipeline::runner::PipelineRunner;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use url::Url;
use uuid::Uuid;

pub async fn start(runner: Arc<PipelineRunner>, config: Amqp) {
    let worker_uuid = Uuid::new_v4();

    let container_id = format!("notifico-worker-{}", worker_uuid);

    match (config.amqp_url, config.amqp_bind) {
        (None, Some(bind)) => {
            let tcp_listener = TcpListener::bind(bind).await.unwrap();
            let connection_acceptor = ConnectionAcceptor::new(container_id);

            info!(
                "Listening for AMQP connections on: {}",
                tcp_listener.local_addr().unwrap()
            );

            while let Ok((stream, addr)) = tcp_listener.accept().await {
                info!("Accepted p2p AMQP connection from: {}", addr);
                let runner = runner.clone();

                let mut connection = connection_acceptor.accept(stream).await.unwrap();
                let _handle = tokio::spawn(async move {
                    let session_acceptor = SessionAcceptor::new();
                    while let Ok(mut session) = session_acceptor.accept(&mut connection).await {
                        let runner = runner.clone();

                        let _handle = tokio::spawn(async move {
                            let link_acceptor = LinkAcceptor::new();
                            match link_acceptor.accept(&mut session).await.unwrap() {
                                LinkEndpoint::Sender(_) => {}
                                LinkEndpoint::Receiver(receiver) => {
                                    let res = process_link(receiver, runner.clone()).await;
                                    if let Err(e) = res {
                                        info!("Error processing AMQP connection: {}", e);
                                    }
                                }
                            }
                        });
                    }
                });
            }
        }
        (Some(url), None) => loop {
            let res =
                connect_to_broker(url.clone(), &url.path(), &container_id, runner.clone()).await;
            if let Err(e) = res {
                info!("Error processing AMQP broker: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        },
        _ => {
            panic!("Invalid AMQP configuration");
        }
    }
}

async fn connect_to_broker(
    url: Url,
    address: &str,
    container_id: &str,
    runner: Arc<PipelineRunner>,
) -> anyhow::Result<()> {
    info!("Connecting to AMQP broker: {}", url);
    let mut connection = Connection::open(container_id, url.clone()).await?;
    info!("Connected to AMQP broker: {}", url);
    let mut session = Session::begin(&mut connection).await?;
    let receiver = Receiver::attach(&mut session, "rust-receiver-link-1", address).await?;
    process_link(receiver, runner).await
}

async fn process_link(mut receiver: Receiver, runner: Arc<PipelineRunner>) -> anyhow::Result<()> {
    loop {
        let delivery = receiver.recv::<String>().await?;

        receiver.accept(&delivery).await?;
        let eventrequest = serde_json::from_str(delivery.body())?;
        runner.process_eventrequest(eventrequest).await;
    }
}
