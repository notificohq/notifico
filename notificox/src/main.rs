use clap::{Parser, Subcommand};
use log::info;
use notifico_core::contact::RawContact;
use notifico_core::credentials::memory::MemoryCredentialStorage;
use notifico_core::credentials::RawCredential;
use notifico_core::engine::plugin::core::CorePlugin;
use notifico_core::engine::Engine;
use notifico_core::pipeline::event::{EventHandler, ProcessEventRequest, RecipientSelector};
use notifico_core::pipeline::executor::PipelineExecutor;
use notifico_core::pipeline::storage::SinglePipelineStorage;
use notifico_core::pipeline::Pipeline;
use notifico_core::recipient::Recipient;
use notifico_core::recorder::BaseRecorder;
use notifico_core::step::SerializedStep;
use notifico_core::transport::TransportRegistry;
use notifico_template::source::DummyTemplateSource;
use notifico_template::Templater;
use notifico_transports::all_transports;
use serde_json::{json, Map, Value};
use std::str::FromStr;
use std::sync::Arc;
use tokio::task::JoinSet;
use url::Url;
use uuid::Uuid;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Send a notification locally (simple syntax)
    Send {
        /// Credential string, transport-specific (refer to the documentation for specific transport)
        credential: String,
        /// Recipient(s), can be an email, phone number, or any other unique identifier
        /// in following format: "TYPE:VALUE"
        contacts: Vec<String>,
        /// Template object in JSON5 format (can be used without escaping)
        #[arg(short, long, required = true)]
        template: Vec<String>,
    },
    /// Send an event to remote Notifico Ingest API
    SendEvent {
        /// URL of the Notifico Ingest API
        #[arg(short, long)]
        ingest: Url,
        /// Event name
        event: String,
        /// Recipient in JSON5 format (can be used without escaping). Refer to documentation for recipient schema.
        #[arg(short, long)]
        recipient: Vec<String>,
        /// Context in JSON5 format (can be used without escaping).
        context: String,
    },
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "notificox=info,warn");
    }

    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Send {
            credential,
            contacts,
            template,
        } => {
            let mut engine = Engine::new();
            let mut transport_registry = TransportRegistry::new();
            let recorder = Arc::new(BaseRecorder::new());

            let credential = RawCredential::from_str(&credential).unwrap();

            let credentials = {
                let mut credentials = MemoryCredentialStorage::default();
                credentials.add_credential(
                    Uuid::nil(),
                    "notificox".to_string(),
                    credential.clone(),
                );
                Arc::new(credentials)
            };

            for (engine_plugin, transport_plugin) in
                all_transports(credentials.clone(), recorder.clone())
            {
                engine.add_plugin(engine_plugin);
                transport_registry.register(transport_plugin);
            }

            let pipeline = {
                let mut pipeline = Pipeline::default();

                if !template.is_empty() {
                    let templates: Vec<Map<String, Value>> = template
                        .iter()
                        .map(|s| json5::from_str(s).unwrap())
                        .collect();

                    let step = json!({
                        "step": "templates.load",
                        "templates": templates,
                    });

                    let step = SerializedStep(step.as_object().cloned().unwrap());
                    pipeline.steps.push(step);
                }
                let transport_name = credential.transport;
                let step = SerializedStep(
                    json!({ "step": transport_registry.get_step(&transport_name).unwrap(), "credential": "notificox" })
                        .as_object()
                        .cloned()
                        .unwrap(),
                );
                pipeline.steps.push(step);
                pipeline
            };

            let contacts: Vec<RawContact> = contacts.iter().map(|s| s.parse().unwrap()).collect();

            let recipient = Recipient {
                id: Uuid::nil(),
                contacts,
            };

            let process_event_request = ProcessEventRequest {
                id: Uuid::nil(),
                project_id: Uuid::nil(),
                event: "notificox".to_string(),
                recipients: vec![RecipientSelector::Recipient(recipient)],
                context: Default::default(),
            };

            info!(
                "Running pipeline: {}",
                serde_json::to_string_pretty(&pipeline).unwrap()
            );

            let (pipelines_tx, pipelines_rx) = flume::unbounded();
            let pipelines_tx = Arc::new(pipelines_tx);

            engine.add_plugin(Arc::new(CorePlugin::new(pipelines_tx.clone())));

            let templater_source = Arc::new(DummyTemplateSource);
            engine.add_plugin(Arc::new(Templater::new(templater_source.clone())));

            // Create PipelineExecutor
            let executor = Arc::new(PipelineExecutor::new(engine));
            let pipelines = Arc::new(SinglePipelineStorage::new(pipeline.clone()));
            let event_handler = EventHandler::new(pipelines.clone(), pipelines_tx.clone());

            event_handler
                .process_eventrequest(process_event_request)
                .await
                .unwrap();

            let mut joinset = JoinSet::new();
            loop {
                tokio::select! {
                    biased;
                    Ok(task) = pipelines_rx.recv_async() => {
                        let executor = executor.clone();
                        let _handle = joinset.spawn(async move {executor.execute_pipeline(serde_json::from_str(&task).unwrap()).await});
                    }
                    result = joinset.join_next() => {
                        if result.is_none() { break;}
                    },
                }
            }
        }
        Command::SendEvent {
            ingest,
            event,
            recipient,
            context,
        } => {
            let recipients: Vec<RecipientSelector> = recipient
                .iter()
                .map(|s| json5::from_str(s).unwrap())
                .collect();

            let request = ProcessEventRequest {
                id: Uuid::now_v7(),
                project_id: Default::default(),
                event,
                recipients,
                context: json5::from_str(&context).unwrap(),
            };

            let url = ingest.join("/v1/send").unwrap();

            let client = reqwest::Client::new();
            client.post(url).json(&request).send().await.unwrap();
        }
    }
}
