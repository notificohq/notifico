mod admin;
mod auth;
mod broadcast;
mod config;
mod ingest;
mod metrics;
mod openapi;
mod public;
mod rate_limit;
mod worker;

use std::sync::Arc;

use axum::{Router, extract::State, middleware, routing::{get, post}};
use sea_orm::DatabaseConnection;
use tower_http::trace::TraceLayer;

use config::{Config, ServerMode};
use notifico_core::registry::TransportRegistry;
use notifico_core::transport::ConsoleTransport;
use notifico_core::transport::email::EmailTransport;

pub(crate) struct AppState {
    pub(crate) db: DatabaseConnection,
    pub(crate) config: Config,
    pub(crate) registry: TransportRegistry,
    pub(crate) encryption_key: Option<[u8; 32]>,
    pub(crate) metrics_handle: Option<metrics_exporter_prometheus::PrometheusHandle>,
    pub(crate) rate_limiter: rate_limit::RateLimiter,
}

#[tokio::main]
async fn main() {
    let config = Config::load(None).expect("Failed to load configuration");

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".parse().unwrap());

    match config.server.log_format {
        config::LogFormat::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .init();
        }
        config::LogFormat::Text => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .init();
        }
    }

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
    registry.register(Arc::new(EmailTransport));

    // Parse encryption key from config (hex-encoded 32-byte key)
    let encryption_key = config.auth.encryption_key.as_ref().map(|hex_key| {
        let bytes = hex::decode(hex_key).expect("NOTIFICO_AUTH_ENCRYPTION_KEY must be valid hex");
        let key: [u8; 32] = bytes
            .try_into()
            .expect("NOTIFICO_AUTH_ENCRYPTION_KEY must be exactly 32 bytes (64 hex chars)");
        key
    });

    let metrics_handle = metrics::install_prometheus_recorder();

    let state = Arc::new(AppState {
        db,
        config: config.clone(),
        registry,
        encryption_key,
        metrics_handle: Some(metrics_handle),
        rate_limiter: rate_limit::RateLimiter::new(100, 60),
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
        .route("/ready", get(health))
        .route("/metrics", get(metrics::metrics_handler))
        .route("/api/v1/events", post(ingest::handle_ingest))
        .route("/api/v1/broadcasts", post(broadcast::handle_broadcast))
        .route("/api/openapi.json", get(openapi::openapi_json))
        .nest("/admin/api/v1", admin::admin_router())
        .nest("/api/v1/public", public::public_router())
        .layer(middleware::from_fn(metrics::track_metrics))
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

async fn health(
    State(state): State<Arc<AppState>>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use sea_orm::ConnectionTrait;
    match state.db.execute_unprepared("SELECT 1").await {
        Ok(_) => (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({"status": "ok", "db": "connected"})),
        ),
        Err(_) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({"status": "degraded", "db": "unreachable"})),
        ),
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
            encryption_key: None,
            metrics_handle: None,
            rate_limiter: rate_limit::RateLimiter::new(1000, 60),
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

    async fn setup_admin_app() -> (Router, String) {
        let db = notifico_db::connect("sqlite::memory:").await.unwrap();
        notifico_db::run_migrations(&db).await.unwrap();

        let project_id = Uuid::now_v7();

        // Seed project
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();

        // Seed admin API key
        let raw_key = "nk_live_admin_test_key_1234";
        notifico_db::repo::api_key::insert_api_key(
            &db,
            Uuid::now_v7(),
            project_id,
            "Admin Key",
            raw_key,
            "admin",
        )
        .await
        .unwrap();

        let config = Config::load(None).unwrap();
        let registry = TransportRegistry::new();

        let state = Arc::new(AppState {
            db,
            config,
            registry,
            encryption_key: None,
            metrics_handle: None,
            rate_limiter: rate_limit::RateLimiter::new(1000, 60),
        });

        (build_router(state), raw_key.to_string())
    }

    async fn json_body(resp: axum::http::Response<Body>) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn admin_project_crud() {
        let (app, key) = setup_admin_app().await;

        // Create project
        let req = Request::builder()
            .method("POST")
            .uri("/admin/api/v1/projects")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(r#"{"name":"My Project"}"#))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        assert_eq!(body["name"], "My Project");
        assert_eq!(body["default_locale"], "en");
        let project_id = body["id"].as_str().unwrap().to_string();

        // List projects (should include the seeded one + newly created)
        let req = Request::builder()
            .uri("/admin/api/v1/projects")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 2);

        // Get project
        let req = Request::builder()
            .uri(format!("/admin/api/v1/projects/{project_id}"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["name"], "My Project");

        // Update project
        let req = Request::builder()
            .method("PUT")
            .uri(format!("/admin/api/v1/projects/{project_id}"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"name":"Updated","default_locale":"fr"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["name"], "Updated");
        assert_eq!(body["default_locale"], "fr");

        // Delete project
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/admin/api/v1/projects/{project_id}"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn admin_event_and_rule_crud() {
        let (app, key) = setup_admin_app().await;

        // Create event
        let req = Request::builder()
            .method("POST")
            .uri("/admin/api/v1/events")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"name":"order.created","category":"transactional"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        let event_id = body["id"].as_str().unwrap().to_string();

        // Create template (needed for rule)
        let req = Request::builder()
            .method("POST")
            .uri("/admin/api/v1/templates")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"name":"order_tpl","channel":"email"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        let template_id = body["id"].as_str().unwrap().to_string();

        // Create rule
        let req = Request::builder()
            .method("POST")
            .uri(format!("/admin/api/v1/events/{event_id}/rules"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(format!(
                r#"{{"channel":"email","template_id":"{template_id}","priority":5}}"#
            )))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        assert_eq!(body["channel"], "email");
        assert_eq!(body["enabled"], true);
        let rule_id = body["id"].as_str().unwrap().to_string();

        // List rules
        let req = Request::builder()
            .uri(format!("/admin/api/v1/events/{event_id}/rules"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 1);

        // Delete rule
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/admin/api/v1/rules/{rule_id}"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn admin_recipient_and_contact_crud() {
        let (app, key) = setup_admin_app().await;

        // Create recipient
        let req = Request::builder()
            .method("POST")
            .uri("/admin/api/v1/recipients")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(r#"{"external_id":"user-42","locale":"fr"}"#))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        assert_eq!(body["external_id"], "user-42");
        assert_eq!(body["locale"], "fr");
        let recipient_id = body["id"].as_str().unwrap().to_string();

        // Add contact
        let req = Request::builder()
            .method("POST")
            .uri(format!("/admin/api/v1/recipients/{recipient_id}/contacts"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"channel":"email","value":"user@test.com"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        let contact_id = body["id"].as_str().unwrap().to_string();
        assert_eq!(body["channel"], "email");

        // List contacts
        let req = Request::builder()
            .uri(format!("/admin/api/v1/recipients/{recipient_id}/contacts"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 1);

        // Delete contact
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/admin/api/v1/contacts/{contact_id}"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Delete recipient
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/admin/api/v1/recipients/{recipient_id}"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn admin_api_keys_and_channels() {
        let (app, key) = setup_admin_app().await;

        // Create API key
        let req = Request::builder()
            .method("POST")
            .uri("/admin/api/v1/api-keys")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(r#"{"name":"Ingest Key"}"#))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        assert_eq!(body["name"], "Ingest Key");
        assert_eq!(body["scope"], "ingest");
        assert!(body["raw_key"].as_str().unwrap().starts_with("nk_live_"));
        let key_id = body["id"].as_str().unwrap().to_string();

        // List API keys (should have 2: admin + new ingest)
        let req = Request::builder()
            .uri("/admin/api/v1/api-keys")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 2);

        // Delete API key
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/admin/api/v1/api-keys/{key_id}"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // List channels (empty since no transports registered in test)
        let req = Request::builder()
            .uri("/admin/api/v1/channels")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert!(body.as_array().unwrap().is_empty());

        // Delivery log (empty)
        let req = Request::builder()
            .uri("/admin/api/v1/delivery-log")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["total"], 0);
        assert!(body["items"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn admin_requires_admin_scope() {
        let (app, ingest_key) = setup_app().await;

        // Ingest-scoped key should get 403 on admin endpoints
        let req = Request::builder()
            .uri("/admin/api/v1/projects")
            .header("authorization", format!("Bearer {ingest_key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn public_preferences_and_unsubscribe() {
        let (app, api_key) = setup_app().await;

        // First, ingest an event to create the recipient "user-123"
        let ingest_body = serde_json::json!({
            "event": "order.confirmed",
            "recipients": [{"id": "user-123", "contacts": {"email": "test@example.com"}}],
            "data": {"order_id": 1, "name": "Test"}
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/events")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {api_key}"))
            .body(Body::from(serde_json::to_string(&ingest_body).unwrap()))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Set a preference (opt out of marketing email)
        let req = Request::builder()
            .method("PUT")
            .uri("/api/v1/public/preferences")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {api_key}"))
            .body(Body::from(serde_json::to_string(&serde_json::json!({
                "recipient": "user-123",
                "category": "marketing",
                "channel": "email",
                "enabled": false
            })).unwrap()))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Get preferences
        let req = Request::builder()
            .uri("/api/v1/public/preferences?recipient=user-123")
            .header("authorization", format!("Bearer {api_key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        let prefs = body.as_array().unwrap();
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0]["category"], "marketing");
        assert_eq!(prefs[0]["enabled"], false);

        // Re-enable preference
        let req = Request::builder()
            .method("PUT")
            .uri("/api/v1/public/preferences")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {api_key}"))
            .body(Body::from(serde_json::to_string(&serde_json::json!({
                "recipient": "user-123",
                "category": "marketing",
                "channel": "email",
                "enabled": true
            })).unwrap()))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn public_unsubscribe_via_token() {
        let (app, _) = setup_app().await;

        // Unsubscribe with an invalid token (POST)
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/public/unsubscribe")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"token":"invalid_token"}"#))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["unsubscribed"], false);

        // Unsubscribe with an invalid token (GET)
        let req = Request::builder()
            .uri("/api/v1/public/unsubscribe?token=invalid_token")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
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

    #[tokio::test]
    async fn health_returns_ok_with_db_status() {
        let (app, _) = setup_app().await;

        let req = Request::builder()
            .method("GET")
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = json_body(resp).await;
        assert_eq!(body["status"], "ok");
        assert_eq!(body["db"], "connected");
    }

    #[tokio::test]
    async fn broadcast_sends_to_all_recipients() {
        let (app, api_key) = setup_app().await;

        // First, ingest events to create two recipients with contacts
        for user in &["bcast-user-1", "bcast-user-2"] {
            let body = serde_json::json!({
                "event": "order.confirmed",
                "recipients": [
                    {"id": user, "contacts": {"email": format!("{}@example.com", user)}}
                ],
                "data": {"order_id": 1}
            });
            let req = Request::builder()
                .method("POST")
                .uri("/api/v1/events")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {api_key}"))
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }

        // Now broadcast to all recipients
        let body = serde_json::json!({
            "event": "order.confirmed",
            "data": {"order_id": 99}
        });
        let req = Request::builder()
            .method("POST")
            .uri("/api/v1/broadcasts")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {api_key}"))
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = json_body(resp).await;
        assert!(body["broadcast_id"].is_string());
        assert!(body["recipient_count"].as_u64().unwrap() >= 2);
        assert!(body["task_count"].as_u64().unwrap() >= 2);
    }

    #[tokio::test]
    async fn openapi_spec_is_valid() {
        let (app, _) = setup_app().await;

        let req = Request::builder()
            .method("GET")
            .uri("/api/openapi.json")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = json_body(resp).await;
        assert_eq!(body["info"]["title"], "Notifico API");
        assert!(body["paths"]["/api/v1/events"].is_object());
        assert!(body["paths"]["/api/v1/broadcasts"].is_object());
    }
}
