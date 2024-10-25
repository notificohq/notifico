use fe2o3_amqp::acceptor::{ConnectionAcceptor, LinkAcceptor, LinkEndpoint, SessionAcceptor};
use fe2o3_amqp::{Connection, Receiver, Session};
use notifico_core::config::Amqp;
use notifico_core::pipeline::runner::PipelineRunner;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use uuid::Uuid;

pub async fn start(runner: Arc<PipelineRunner>, config: Amqp) {
    let worker_uuid = Uuid::new_v4();

    let container_id = format!("notifico-worker-{}", worker_uuid);

    match config {
        Amqp::Bind { bind } => {
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
                                LinkEndpoint::Receiver(recver) => {
                                    process_link(recver, runner).await
                                }
                            }
                        });
                    }
                });
            }
        }
        Amqp::Broker { url, queue } => {
            let mut connection = Connection::open(container_id, url.clone()).await.unwrap();

            let mut session = Session::begin(&mut connection).await.unwrap();

            let receiver = Receiver::attach(&mut session, "rust-receiver-link-1", queue.clone())
                .await
                .unwrap();

            process_link(receiver, runner).await;
        }
    }
}

async fn process_link(mut receiver: Receiver, runner: Arc<PipelineRunner>) {
    loop {
        if let Ok(delivery) = receiver.recv::<String>().await {
            receiver.accept(&delivery).await.unwrap();

            let eventrequest = serde_json::from_str(delivery.body()).unwrap();

            runner.process_eventrequest(eventrequest).await;
        } else {
            break;
        }
    }
}
