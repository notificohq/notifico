mod config;
mod worker;

use std::sync::Arc;

use axum::{Router, extract::State, routing::get};
use sea_orm::DatabaseConnection;
use tower_http::trace::TraceLayer;

use config::{Config, ServerMode};
use notifico_core::registry::TransportRegistry;

struct AppState {
    db: DatabaseConnection,
    config: Config,
    registry: TransportRegistry,
}

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
        db = config.database.url.as_str(),
        "Notifico v2 starting"
    );

    // Connect to database
    let db = notifico_db::connect(&config.database.url)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Connected to database");

    // Run migrations
    notifico_db::run_migrations(&db)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database migrations complete");

    let registry = TransportRegistry::new();

    let state = Arc::new(AppState {
        db,
        config: config.clone(),
        registry,
    });

    match config.server.mode {
        ServerMode::All | ServerMode::Api => {
            start_api_server(state).await;
        }
        ServerMode::Worker => {
            tracing::info!("Worker mode — HTTP server not started");
            // Worker loop will go here in future phases
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl+c");
        }
    }
}

async fn start_api_server(state: Arc<AppState>) {
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    tracing::info!(addr = %addr, "API server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

async fn health() -> &'static str {
    "ok"
}

async fn ready(State(state): State<Arc<AppState>>) -> (axum::http::StatusCode, &'static str) {
    // Check DB connectivity
    use sea_orm::ConnectionTrait;
    match state.db.execute_unprepared("SELECT 1").await {
        Ok(_) => (axum::http::StatusCode::OK, "ready"),
        Err(_) => (axum::http::StatusCode::SERVICE_UNAVAILABLE, "not ready"),
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl+c");
    tracing::info!("Shutdown signal received");
}
