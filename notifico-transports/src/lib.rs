use notifico_attachment::AttachmentPlugin;
use notifico_core::credentials::CredentialStorage;
use notifico_core::engine::EnginePlugin;
use notifico_core::recorder::Recorder;
use notifico_core::simpletransport::SimpleTransportWrapper;
use notifico_core::transport::Transport;
use notifico_gotify::GotifyTransport;
use notifico_ntfy::NtfyTransport;
use notifico_pushover::PushoverTransport;
use notifico_slack::SlackTransport;
use notifico_smpp::SmppPlugin;
use notifico_smtp::EmailPlugin;
use notifico_telegram::TelegramTransport;
use notifico_whatsapp::WabaTransport;
use std::sync::Arc;

pub fn all_transports(
    credentials: Arc<dyn CredentialStorage>,
    recorder: Arc<dyn Recorder>,
    attachments: Arc<AttachmentPlugin>,
) -> Vec<(Arc<dyn EnginePlugin>, Arc<dyn Transport>)> {
    let mut plugins: Vec<(Arc<dyn EnginePlugin>, Arc<dyn Transport>)> = vec![];
    let http = reqwest::Client::builder().build().unwrap();

    // Complicated transports
    let email_plugin = Arc::new(EmailPlugin::new(credentials.clone(), recorder.clone()));
    plugins.push((email_plugin.clone(), email_plugin.clone()));

    let smpp_plugin = Arc::new(SmppPlugin::new(credentials.clone()));
    plugins.push((smpp_plugin.clone(), smpp_plugin.clone()));

    // Simple transports
    let telegram_transport = Arc::new(TelegramTransport::new(http.clone(), attachments.clone()));
    let telegram_plugin = Arc::new(SimpleTransportWrapper::new(
        telegram_transport,
        credentials.clone(),
        recorder.clone(),
    ));
    plugins.push((telegram_plugin.clone(), telegram_plugin.clone()));

    let waba_transport = Arc::new(WabaTransport::new(http.clone()));
    let waba_plugin = Arc::new(SimpleTransportWrapper::new(
        waba_transport,
        credentials.clone(),
        recorder.clone(),
    ));
    plugins.push((waba_plugin.clone(), waba_plugin.clone()));

    let slack_transport = Arc::new(SlackTransport::new(http.clone()));
    let slack_plugin = Arc::new(SimpleTransportWrapper::new(
        slack_transport,
        credentials.clone(),
        recorder.clone(),
    ));
    plugins.push((slack_plugin.clone(), slack_plugin.clone()));

    let pushover_transport = Arc::new(PushoverTransport::new(http.clone()));
    let pushover_plugin = Arc::new(SimpleTransportWrapper::new(
        pushover_transport,
        credentials.clone(),
        recorder.clone(),
    ));
    plugins.push((pushover_plugin.clone(), pushover_plugin.clone()));

    let gotify_transport = Arc::new(GotifyTransport::new(http.clone()));
    let gotify_plugin = Arc::new(SimpleTransportWrapper::new(
        gotify_transport,
        credentials.clone(),
        recorder.clone(),
    ));
    plugins.push((gotify_plugin.clone(), gotify_plugin.clone()));

    let ntfy_transport = Arc::new(NtfyTransport::new(http.clone(), attachments.clone()));
    let ntfy_plugin = Arc::new(SimpleTransportWrapper::new(
        ntfy_transport,
        credentials.clone(),
        recorder.clone(),
    ));
    plugins.push((ntfy_plugin.clone(), ntfy_plugin.clone()));

    // Add more transports here...

    plugins
}
