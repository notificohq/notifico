mod admin;
mod auth;
mod broadcast;
mod config;
mod frontend;
mod ingest;
mod metrics;
mod openapi;
mod public;
mod rate_limit;
mod tracking;
mod worker;

use std::sync::Arc;

use axum::{Router, extract::State, middleware, routing::{get, post}};
use sea_orm::DatabaseConnection;
use tower_http::trace::TraceLayer;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use config::{Config, ServerMode};
use notifico_core::middleware::MiddlewareRegistry;
use notifico_core::middleware::click_tracking::ClickTrackingMiddleware;
use notifico_core::middleware::open_tracking::OpenTrackingMiddleware;
use notifico_core::middleware::plaintext_fallback::PlaintextFallbackMiddleware;
use notifico_core::middleware::unsubscribe_link::UnsubscribeLinkMiddleware;
use notifico_core::middleware::utm_params::UtmParamsMiddleware;
use notifico_core::registry::TransportRegistry;
use notifico_transport_console::ConsoleTransport;
use notifico_transport_discord::DiscordTransport;
use notifico_transport_email::EmailTransport;
use notifico_transport_slack::SlackTransport;
use notifico_transport_twilio_sms::TwilioSmsTransport;
use notifico_transport_telegram::TelegramTransport;
use notifico_transport_webhook::WebhookTransport;
use notifico_transport_fcm::FcmTransport;

pub(crate) struct AppState {
    pub(crate) db: DatabaseConnection,
    pub(crate) config: Config,
    pub(crate) registry: TransportRegistry,
    pub(crate) middleware_registry: MiddlewareRegistry,
    pub(crate) encryption_key: Option<[u8; 32]>,
    pub(crate) metrics_handle: Option<metrics_exporter_prometheus::PrometheusHandle>,
    pub(crate) rate_limiter: rate_limit::RateLimiter,
}

#[tokio::main]
async fn main() {
    let config = Config::load(None).expect("Failed to load configuration");

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".parse().unwrap());

    // Build optional OpenTelemetry layer
    let otel_layer = if let Some(ref endpoint) = config.otel.endpoint {
        use opentelemetry::trace::TracerProvider;
        use opentelemetry_otlp::WithExportConfig;

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()
            .expect("Failed to create OTLP span exporter");

        let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(
                opentelemetry_sdk::Resource::builder()
                    .with_service_name(config.otel.service_name.clone())
                    .build(),
            )
            .build();

        let tracer = tracer_provider.tracer("notifico");
        // Register the global provider so it can be shut down later
        opentelemetry::global::set_tracer_provider(tracer_provider);

        eprintln!("OpenTelemetry OTLP exporter configured for endpoint: {endpoint}");
        Some(tracing_opentelemetry::layer().with_tracer(tracer))
    } else {
        None
    };

    match config.server.log_format {
        config::LogFormat::Json => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(otel_layer)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        config::LogFormat::Text => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(otel_layer)
                .with(tracing_subscriber::fmt::layer())
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
    registry.register(Arc::new(SlackTransport::new()));
    registry.register(Arc::new(DiscordTransport::new()));
    registry.register(Arc::new(TwilioSmsTransport::new()));
    registry.register(Arc::new(TelegramTransport::new()));
    registry.register(Arc::new(WebhookTransport::new()));
    registry.register(Arc::new(FcmTransport::new()));

    // Parse encryption key from config (hex-encoded 32-byte key)
    let encryption_key = config.auth.encryption_key.as_ref().map(|hex_key| {
        let bytes = hex::decode(hex_key).expect("NOTIFICO_AUTH_ENCRYPTION_KEY must be valid hex");
        let key: [u8; 32] = bytes
            .try_into()
            .expect("NOTIFICO_AUTH_ENCRYPTION_KEY must be exactly 32 bytes (64 hex chars)");
        key
    });

    let metrics_handle = metrics::install_prometheus_recorder();

    let mut middleware_registry = MiddlewareRegistry::new();
    middleware_registry.register(Arc::new(UnsubscribeLinkMiddleware));
    middleware_registry.register(Arc::new(ClickTrackingMiddleware));
    middleware_registry.register(Arc::new(OpenTrackingMiddleware));
    middleware_registry.register(Arc::new(UtmParamsMiddleware));
    middleware_registry.register(Arc::new(PlaintextFallbackMiddleware));

    let state = Arc::new(AppState {
        db,
        config: config.clone(),
        registry,
        middleware_registry,
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
        .merge(openapi::swagger_ui_router())
        .nest("/admin/api/v1", admin::admin_router())
        .nest("/api/v1/public", public::public_router())
        .route("/t/open/{token}", get(tracking::handle_open))
        .route("/t/click/{token}", get(tracking::handle_click))
        .fallback(frontend::serve_frontend)
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
            middleware_registry: MiddlewareRegistry::new(),
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
            middleware_registry: MiddlewareRegistry::new(),
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

    #[tokio::test]
    async fn template_preview_renders_without_sending() {
        let (app, key) = setup_admin_app().await;

        // Create template
        let req = Request::builder()
            .method("POST")
            .uri("/admin/api/v1/templates")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(r#"{"name":"preview_tpl","channel":"email"}"#))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        let template_id = body["id"].as_str().unwrap().to_string();

        // Set template content (wrapped in "body" field per SetContentRequest)
        let req = Request::builder()
            .method("PUT")
            .uri(format!(
                "/admin/api/v1/templates/{template_id}/content/en"
            ))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"body":{"subject":"Hello {{ name }}","text":"Order #{{ order_id }} confirmed"}}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Preview the template
        let req = Request::builder()
            .method("POST")
            .uri(format!("/admin/api/v1/templates/{template_id}/preview"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"locale":"en","data":{"name":"Alice","order_id":42}}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = json_body(resp).await;
        assert_eq!(body["rendered"]["subject"], "Hello Alice");
        assert_eq!(body["rendered"]["text"], "Order #42 confirmed");
    }

    #[tokio::test]
    async fn event_stats_returns_delivery_counts() {
        let (app, key) = setup_admin_app().await;

        // Create event
        let req = Request::builder()
            .method("POST")
            .uri("/admin/api/v1/events")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"name":"stats.test","category":"transactional"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        let event_id = body["id"].as_str().unwrap().to_string();

        // Get stats (should be empty initially)
        let req = Request::builder()
            .uri(format!("/admin/api/v1/events/{event_id}/stats"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = json_body(resp).await;
        assert_eq!(body["event_id"], event_id);
        assert!(body["stats"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn admin_middleware_crud() {
        let (app, key) = setup_admin_app().await;

        // Create event
        let req = Request::builder()
            .method("POST")
            .uri("/admin/api/v1/events")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"name":"mw.test","category":"transactional"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        let event_id = body["id"].as_str().unwrap().to_string();

        // Create template
        let req = Request::builder()
            .method("POST")
            .uri("/admin/api/v1/templates")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"name":"mw_tpl","channel":"email"}"#,
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
        let rule_id = body["id"].as_str().unwrap().to_string();

        // POST middleware to rule -> 201
        let req = Request::builder()
            .method("POST")
            .uri(format!("/admin/api/v1/rules/{rule_id}/middleware"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"middleware_name":"rate_limiter","config":{"max":100},"priority":10}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = json_body(resp).await;
        assert_eq!(body["middleware_name"], "rate_limiter");
        assert_eq!(body["priority"], 10);
        assert_eq!(body["enabled"], true);
        let mw_id = body["id"].as_str().unwrap().to_string();

        // GET middleware list for rule -> should have 1
        let req = Request::builder()
            .uri(format!("/admin/api/v1/rules/{rule_id}/middleware"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 1);

        // PUT update middleware -> 200
        let req = Request::builder()
            .method("PUT")
            .uri(format!("/admin/api/v1/middleware/{mw_id}"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {key}"))
            .body(Body::from(
                r#"{"config":{"max":200},"priority":20,"enabled":false}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body["updated"], true);

        // DELETE middleware -> 204
        let req = Request::builder()
            .method("DELETE")
            .uri(format!("/admin/api/v1/middleware/{mw_id}"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // GET middleware list -> should be empty
        let req = Request::builder()
            .uri(format!("/admin/api/v1/rules/{rule_id}/middleware"))
            .header("authorization", format!("Bearer {key}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json_body(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 0);
    }

    async fn setup_tracking_app() -> (Router, [u8; 32]) {
        let db = notifico_db::connect("sqlite::memory:").await.unwrap();
        notifico_db::run_migrations(&db).await.unwrap();

        let config = Config::load(None).unwrap();
        let registry = TransportRegistry::new();
        let key: [u8; 32] = [0xAB; 32];

        let state = Arc::new(AppState {
            db,
            config,
            registry,
            middleware_registry: MiddlewareRegistry::new(),
            encryption_key: Some(key),
            metrics_handle: None,
            rate_limiter: rate_limit::RateLimiter::new(1000, 60),
        });

        (build_router(state), key)
    }

    #[tokio::test]
    async fn tracking_open_returns_gif() {
        let (app, key) = setup_tracking_app().await;

        let token = tracking::create_tracking_token("dlv-001", None, &key);
        let req = Request::builder()
            .uri(format!("/t/open/{token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "image/gif"
        );
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.len(), 43);
    }

    #[tokio::test]
    async fn tracking_open_invalid_token_still_returns_gif() {
        let (app, _key) = setup_tracking_app().await;

        let req = Request::builder()
            .uri("/t/open/invalid_token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "image/gif"
        );
    }

    #[tokio::test]
    async fn tracking_click_redirects() {
        let (app, key) = setup_tracking_app().await;

        let url = "https://example.com/landing";
        let token = tracking::create_tracking_token("dlv-002", Some(url), &key);
        let req = Request::builder()
            .uri(format!("/t/click/{token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            resp.headers().get("location").unwrap(),
            url
        );
    }

    #[tokio::test]
    async fn tracking_click_invalid_token_returns_400() {
        let (app, _key) = setup_tracking_app().await;

        let req = Request::builder()
            .uri("/t/click/invalid_token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
