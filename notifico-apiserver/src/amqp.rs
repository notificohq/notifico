use fe2o3_amqp::{Connection, Sender, Session};
use notifico_core::config::Amqp;
use notifico_core::pipeline::runner::ProcessEventRequest;
use tokio::sync::mpsc::Receiver;
use tracing::info;

//TODO: reconnect to AMQP on failure
pub async fn run(config: Amqp, mut event_rx: Receiver<ProcessEventRequest>) {
    let mut connection = Connection::open("connection-1", config.connection_url())
        .await
        .unwrap();

    let mut session = Session::begin(&mut connection).await.unwrap();

    let address = match config {
        Amqp::Bind { .. } => String::default(),
        Amqp::Broker { address, .. } => address,
    };

    let mut sender = Sender::attach(&mut session, "rust-sender-link-1", address)
        .await
        .unwrap();

    loop {
        tokio::select! {
            req = event_rx.recv() => {
                info!("Sending message");
                let msg = serde_json::to_string(&req).unwrap();
                let _outcome = sender.send(msg).await.unwrap();
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down");
                break;
            }
        }
    }
}
