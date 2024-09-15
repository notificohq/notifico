pub mod engine;

use crate::engine::templater::TemplaterService;
use crate::engine::{Engine, EventContext, PipelineContext, Recipient, Step};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use engine::telegram::TelegramPlugin;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt::init();

    #[derive(OpenApi)]
    #[openapi()]
    struct ApiDoc;

    // build our application with a route
    let app = Router::new()
        .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/trigger", post(trigger));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    futures::future::pending::<()>().await;
}

async fn root() -> &'static str {
    "Hello, World!"
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct TriggerSchema {
    id: Uuid,
    event_id: Uuid,
    context: EventContext,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct TriggerResult {
    id: Uuid,
}

async fn trigger(Json(payload): Json<TriggerSchema>) -> (StatusCode, Json<TriggerResult>) {
    // // // // // // // // // // // // // // // // // // // // // // // // // // // //
    info!("Received context: {:?}", payload.context);

    let pipeline = json!({
        "steps": [
            {
                "type": "telegram.load_template",
                "template_id": "0191d395-f806-7b54-b4db-feffbbe5d2d4"
            },
            {
                "type": "telegram.send",
                "bot_token": ""
            }
        ]
    });

    // Pipeline;
    {
        let templater = Arc::new(TemplaterService::new("http://127.0.0.1:8000"));

        let mut engine = Engine::new();

        let tg_plugin = TelegramPlugin::new(templater);
        engine.add_plugin(tg_plugin);

        let mut context = PipelineContext::default();
        context.recipients = vec![Recipient {
            telegram_id: 111579711i64,
        }];
        context.event_context = payload.context;

        for step in pipeline["steps"].as_array().unwrap().iter() {
            let step_parsed = serde_json::from_value::<Step>(step.clone()).unwrap().r#type;
            engine
                .execute_step(&mut context, &step_parsed, step.clone())
                .await
                .unwrap()
        }
    }

    (StatusCode::CREATED, Json(TriggerResult { id: payload.id }))
}
