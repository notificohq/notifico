use crate::event_handler::{EventHandler, ProcessEvent};
use actix::Addr;
use axum::extract::State;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

pub(crate) async fn start(event_handler: Addr<EventHandler>) {
    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/trigger", post(create_user))
        .with_state(event_handler);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

async fn create_user(
    State(event_handler): State<Addr<EventHandler>>,
    Json(payload): Json<ProcessEvent>,
) -> (StatusCode, Json<User>) {
    let user = User {
        id: payload.project_id,
        username: "created user".to_string(),
    };

    event_handler.send(payload).await.unwrap();

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: Uuid,
    username: String,
}
