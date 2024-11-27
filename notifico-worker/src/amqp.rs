use fe2o3_amqp::{Connection, Receiver, Session};
use notifico_core::pipeline::runner::PipelineRunner;
use std::sync::Arc;
use tracing::info;
use url::Url;
use uuid::Uuid;

pub async fn start(runner: Arc<PipelineRunner>, amqp_url: Url, worker_addr: String) {
    let worker_uuid = Uuid::new_v4();

    let container_id = format!("notifico-worker-{}", worker_uuid);
    loop {
        let res = connect_to_broker(
            amqp_url.clone(),
            &worker_addr,
            &container_id,
            runner.clone(),
        )
        .await;
        if let Err(e) = res {
            info!("Error processing AMQP broker: {}", e);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
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
