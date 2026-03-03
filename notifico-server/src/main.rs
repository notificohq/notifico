mod config;

use config::Config;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    let config = Config::load(None).expect("Failed to load configuration");

    tracing::info!(
        mode = ?config.server.mode,
        port = config.server.port,
        db = config.database.backend.as_str(),
        "Notifico v2 starting"
    );
}
