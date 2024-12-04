use clap::{Parser, Subcommand};
use log::info;
use notifico_core::credentials::DummyCredentialStorage;
use notifico_core::engine::plugin::core::CorePlugin;
use notifico_core::engine::Engine;
use notifico_core::pipeline::event::{EventHandler, ProcessEventRequest};
use notifico_core::pipeline::executor::PipelineExecutor;
use notifico_core::pipeline::storage::SinglePipelineStorage;
use notifico_core::pipeline::Pipeline;
use notifico_core::recorder::BaseRecorder;
use notifico_core::step::SerializedStep;
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
    RunPipeline {
        #[arg(short, long)]
        channel: String,
        #[arg(short, long)]
        recipient: Vec<String>,
        #[arg(short, long)]
        step: Vec<String>,
        #[arg(short, long)]
        template: Vec<String>,
    },
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "notificox=info");
    }

    env_logger::init();

    // tracing_subscriber::registry()
    //     .with(fmt::layer())
    //     .with(EnvFilter::from_default_env())
    //     .init();

    let cli = Cli::parse();
    // println!("{cli:?}");

    match cli.command {
        Command::RunPipeline {
            channel,
            recipient,
            step,
            template,
        } => {
            let mut pipeline = Pipeline::default();

            let recipients = recipient
                .iter()
                .map(|s| json5::from_str(s).unwrap())
                .collect();

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

            for step in step {
                pipeline.steps.push(json5::from_str(&step).unwrap());
            }
            pipeline.channel = channel;

            info!(
                "Running pipeline: {}",
                serde_json::to_string_pretty(&pipeline).unwrap()
            );

            // Create Engine with plugins
            let mut engine = Engine::new();

            let recorder = Arc::new(BaseRecorder::new());

            let (pipelines_tx, pipelines_rx) = flume::unbounded();
            let pipelines_tx = Arc::new(pipelines_tx);

            engine.add_plugin(Arc::new(CorePlugin::new(pipelines_tx.clone())));

            let credentials = Arc::new(DummyCredentialStorage);

            let templater_source = Arc::new(DummyTemplateSource);
            engine.add_plugin(Arc::new(Templater::new(templater_source.clone())));

            engine.add_plugin(Arc::new(TelegramPlugin::new(
                credentials.clone(),
                recorder.clone(),
            )));
            engine.add_plugin(Arc::new(EmailPlugin::new(
                credentials.clone(),
                recorder.clone(),
            )));
            engine.add_plugin(Arc::new(WaBusinessPlugin::new(
                credentials.clone(),
                recorder.clone(),
            )));
            engine.add_plugin(Arc::new(SmppPlugin::new(credentials.clone())));
            engine.add_plugin(Arc::new(SlackPlugin::new(
                credentials.clone(),
                recorder.clone(),
            )));
            engine.add_plugin(Arc::new(PushoverPlugin::new(
                credentials.clone(),
                recorder.clone(),
            )));

            // Create PipelineExecutor
            let executor = Arc::new(PipelineExecutor::new(engine));
            let pipelines = Arc::new(SinglePipelineStorage::new(pipeline.clone()));
            let event_handler = EventHandler::new(pipelines.clone(), pipelines_tx.clone());

            event_handler
                .process_eventrequest(ProcessEventRequest {
                    id: Uuid::nil(),
                    project_id: Uuid::nil(),
                    event: "notificox".to_string(),
                    recipients,
                    context: Default::default(),
                })
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
