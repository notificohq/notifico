use axum::routing::get;
use axum::Router;
use axum_prometheus::metrics_exporter_prometheus::PrometheusHandle;
use axum_prometheus::PrometheusMetricLayer;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub async fn start(bind: SocketAddr, handle: PrometheusHandle) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let listener = TcpListener::bind(bind).await.unwrap();

    // API
    let app = Router::new()
        .route("/metrics", get(|| async move { handle.render() }))
        .layer(PrometheusMetricLayer::new());

    tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
}
