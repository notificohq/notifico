use backoff::future::retry;
use backoff::ExponentialBackoff;
use fe2o3_amqp::{Connection, Sender, Session};
use flume::Receiver;
use notifico_core::pipeline::runner::ProcessEventRequest;
use tracing::{error, info};
use url::Url;

pub async fn run(amqp_url: Url, worker_addr: String, event_rx: Receiver<ProcessEventRequest>) {
    'outer: loop {
        info!("Connecting to AMQP broker: {amqp_url}...");
        let connection = retry(ExponentialBackoff::default(), || async {
            Ok(Connection::open("connection-1", amqp_url.clone()).await?)
        })
        .await;

        let mut connection = match connection {
            Ok(conn) => {
                info!("Connected to AMQP broker: {amqp_url}.");
                conn
            }
            Err(err) => {
                error!("Failed to connect to AMQP broker: {err:?}");
                continue;
            }
        };

        let mut session = match Session::begin(&mut connection).await {
            Ok(session) => session,
            Err(err) => {
                error!("Failed to create session: {err:?}");
                continue;
            }
        };

        let mut sender =
            match Sender::attach(&mut session, "rust-sender-link-1", &worker_addr).await {
                Ok(sender) => sender,
                Err(err) => {
                    error!("Failed to create sender link: {err:?}");
                    continue;
                }
            };

        loop {
            tokio::select! {
                req = event_rx.recv_async() => {
                    let Ok(req) = req else {
                        error!("Event receiver has been closed");
                        break 'outer;
                    };
                    info!("Sending event to AMQP: {req:?}...");

                    let msg = serde_json::to_string(&req).unwrap();
                    let _outcome = sender.send(msg).await.unwrap();
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutting down AMQP session");
                    break 'outer;
                }
            }
        }
    }
}
