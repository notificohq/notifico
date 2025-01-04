mod list_unsubscribe;
mod recipient;

use axum::{Extension, Router};
use notifico_core::http::SecretKey;
use notifico_subscription::controllers::subscription::SubscriptionDbController;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub(crate) struct HttpPublicExtensions {
    pub subscription_controller: Arc<SubscriptionDbController>,
    pub secret_key: Arc<SecretKey>,
}

#[derive(OpenApi)]
#[openapi(info(description = "Notifico User API"))]
struct ApiDoc;

pub(crate) async fn start(bind: SocketAddr, ext: HttpPublicExtensions) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let listener = TcpListener::bind(bind).await.unwrap();

    let app = Router::new().nest("/api", recipient::get_router(ext.clone()));
    let app = app.layer(Extension(ext.secret_key.clone()));

    let app =
        app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
}
