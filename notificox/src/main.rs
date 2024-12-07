use clap::{Parser, Subcommand};
use log::info;
use notifico_core::config::credentials::MemoryCredentialStorage;
use notifico_core::contact::Contact;
use notifico_core::credentials::{Credential, CredentialStorage};
use notifico_core::engine::plugin::core::CorePlugin;
use notifico_core::engine::Engine;
use notifico_core::pipeline::event::{EventHandler, ProcessEventRequest, RecipientSelector};
use notifico_core::pipeline::executor::PipelineExecutor;
use notifico_core::pipeline::storage::SinglePipelineStorage;
use notifico_core::pipeline::Pipeline;
use notifico_core::recipient::Recipient;
use notifico_core::recorder::{BaseRecorder, Recorder};
use notifico_core::simpletransport::SimpleTransportWrapper;
use notifico_core::step::SerializedStep;
use notifico_core::transport::TransportRegistry;
use notifico_gotify::GotifyTransport;
use notifico_pushover::PushoverPlugin;
use notifico_slack::SlackPlugin;
use notifico_smpp::SmppPlugin;
use notifico_smtp::EmailPlugin;
use notifico_telegram::TelegramPlugin;
use notifico_template::source::DummyTemplateSource;
use notifico_template::Templater;
use notifico_whatsapp::WaBusinessPlugin;
use serde_json::{json, Map, Value};
use std::sync::Arc;
use tokio::task::JoinSet;
use uuid::Uuid;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Send {
        #[arg(long)]
        credential: String,
        #[arg(long)]
        to: Vec<String>,
        #[arg(long)]
        channel: String,
        #[arg(long)]
        template: Vec<String>,
    },
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "notificox=info,warn");
    }

    env_logger::init();

    // tracing_subscriber::registry()
    //     .with(fmt::layer())
    //     .with(EnvFilter::from_default_env())
    //     .init();

    let cli = Cli::parse();
    // println!("{cli:?}");

    match cli.command {
        Command::Send {
            credential,
            to,
            channel,
            template,
        } => {
            let mut engine = Engine::new();
            let mut transport_registry = TransportRegistry::new();
            let recorder = Arc::new(BaseRecorder::new());

            let credential = Credential::Short(credential);

            let credentials = {
                let mut credentials = MemoryCredentialStorage::default();
                credentials.add_credential(
                    Uuid::nil(),
                    "notificox".to_string(),
                    credential.clone(),
                );
                Arc::new(credentials)
            };

            add_transports(
                &mut engine,
                &mut transport_registry,
                credentials.clone(),
                recorder.clone(),
            );

            let pipeline = {
                let mut pipeline = Pipeline {
                    channel,
                    ..Default::default()
                };

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
                let transport_name = credential.transport();
                let step = SerializedStep(
                    json!({ "step": transport_registry.get_step(transport_name).unwrap(), "credential": "notificox" })
                        .as_object()
                        .cloned()
                        .unwrap(),
                );
                pipeline.steps.push(step);
                pipeline
            };

            let contacts: Vec<Contact> = {
                let mut contacts = vec![];

                for contact in to {
                    contacts.push(Contact::from_url(&contact).unwrap())
                }

                contacts
            };

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
    }
}

fn add_transports(
    engine: &mut Engine,
    transport_registry: &mut TransportRegistry,
    credentials: Arc<dyn CredentialStorage>,
    recorder: Arc<dyn Recorder>,
) {
    let telegram_plugin = Arc::new(TelegramPlugin::new(credentials.clone(), recorder.clone()));
    engine.add_plugin(telegram_plugin.clone());
    transport_registry.register(telegram_plugin);

    let email_plugin = Arc::new(EmailPlugin::new(credentials.clone(), recorder.clone()));
    engine.add_plugin(email_plugin.clone());
    transport_registry.register(email_plugin);

    let whatsapp_plugin = Arc::new(WaBusinessPlugin::new(credentials.clone(), recorder.clone()));
    engine.add_plugin(whatsapp_plugin.clone());
    transport_registry.register(whatsapp_plugin);

    let smpp_plugin = Arc::new(SmppPlugin::new(credentials.clone()));
    engine.add_plugin(smpp_plugin.clone());
    transport_registry.register(smpp_plugin);

    let slack_plugin = Arc::new(SlackPlugin::new(credentials.clone(), recorder.clone()));
    engine.add_plugin(slack_plugin.clone());
    transport_registry.register(slack_plugin);

    let pushover_plugin = Arc::new(PushoverPlugin::new(credentials.clone(), recorder.clone()));
    engine.add_plugin(pushover_plugin.clone());
    transport_registry.register(pushover_plugin);

    let gotify_transport = Arc::new(GotifyTransport::new());
    let gotify_plugin = Arc::new(SimpleTransportWrapper::new(
        gotify_transport.clone(),
        credentials.clone(),
        recorder.clone(),
    ));
    engine.add_plugin(gotify_plugin.clone());
    transport_registry.register(gotify_plugin);
}
