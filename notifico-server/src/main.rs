mod auth;
mod config;
mod ingest;
mod worker;

use std::sync::Arc;

use axum::{Router, extract::State, routing::{get, post}};
use sea_orm::DatabaseConnection;
use tower_http::trace::TraceLayer;

use config::{Config, ServerMode};
use notifico_core::registry::TransportRegistry;
use notifico_core::transport::ConsoleTransport;

pub(crate) struct AppState {
    pub(crate) db: DatabaseConnection,
    pub(crate) config: Config,
    pub(crate) registry: TransportRegistry,
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

    let mut registry = TransportRegistry::new();
    registry.register(Arc::new(ConsoleTransport));

    let state = Arc::new(AppState {
        db,
        config: config.clone(),
        registry,
    });

    match config.server.mode {
        ServerMode::All => {
            let worker_state = state.clone();
            tokio::spawn(async move {
                worker::run_worker_loop(worker_state).await;
            });
            start_api_server(state).await;
        }
        ServerMode::Api => {
            start_api_server(state).await;
        }
        ServerMode::Worker => {
            tracing::info!("Worker mode — HTTP server not started");
            worker::run_worker_loop(state).await;
        }
    }
}

pub(crate) fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/api/v1/events", post(ingest::handle_ingest))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn start_api_server(state: Arc<AppState>) {
    let app = build_router(state.clone());

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

#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use sea_orm::ConnectionTrait;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn setup_app() -> (Router, String) {
        let db = notifico_db::connect("sqlite::memory:").await.unwrap();
        notifico_db::run_migrations(&db).await.unwrap();

        let project_id = Uuid::now_v7();
        let event_id = Uuid::now_v7();
        let template_id = Uuid::now_v7();
        let version_id = Uuid::now_v7();
        let content_id = Uuid::now_v7();
        let rule_id = Uuid::now_v7();

        // Seed project
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();

        // Seed event
        db.execute_unprepared(&format!(
            "INSERT INTO event (id, project_id, name, category) \
             VALUES ('{event_id}', '{project_id}', 'order.confirmed', 'transactional')"
        ))
        .await
        .unwrap();

        // Seed template
        db.execute_unprepared(&format!(
            "INSERT INTO template (id, project_id, name, channel) \
             VALUES ('{template_id}', '{project_id}', 'order_email', 'email')"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO template_version (id, template_id, version, is_current) \
             VALUES ('{version_id}', '{template_id}', 1, true)"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            r#"INSERT INTO template_content (id, template_version_id, locale, body) VALUES ('{content_id}', '{version_id}', 'en', '{{"subject": "Order #{{{{ order_id }}}}", "text": "Hello {{{{ name }}}}"}}')"#
        ))
        .await
        .unwrap();

        // Seed pipeline rule
        db.execute_unprepared(&format!(
            "INSERT INTO pipeline_rule (id, event_id, channel, template_id, enabled, priority) \
             VALUES ('{rule_id}', '{event_id}', 'email', '{template_id}', true, 10)"
        ))
        .await
        .unwrap();

        // Seed API key
        let raw_key = "nk_live_integration_test_key_1234";
        notifico_db::repo::api_key::insert_api_key(
            &db,
            Uuid::now_v7(),
            project_id,
            "Test Key",
            raw_key,
            "ingest",
        )
        .await
        .unwrap();

        let config = Config::load(None).unwrap();
        let registry = TransportRegistry::new();

        let state = Arc::new(AppState {
            db,
            config,
            registry,
        });

        (build_router(state), raw_key.to_string())
    }

    #[tokio::test]
    async fn ingest_event_end_to_end() {
        let (app, api_key) = setup_app().await;

        let body = serde_json::json!({
            "event": "order.confirmed",
            "recipients": [
                {"id": "user-123", "contacts": {"email": "test@example.com"}}
            ],
            "data": {"order_id": 42, "name": "Alice"}
        });

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/events")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {api_key}"))
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["accepted"], 1);
        assert_eq!(json["task_ids"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn ingest_without_auth_returns_401() {
        let (app, _) = setup_app().await;

        let body = serde_json::json!({
            "event": "order.confirmed",
            "recipients": [{"id": "user-1"}],
            "data": {}
        });

        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/events")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
