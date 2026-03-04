use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{MatchedPath, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use metrics::{counter, histogram};

use crate::AppState;

/// Axum middleware that records HTTP request metrics.
pub async fn track_metrics(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    let response = next.run(req).await;

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    counter!("http_requests_total", "method" => method.to_string(), "path" => path.clone(), "status" => status.clone())
        .increment(1);
    histogram!("http_request_duration_seconds", "method" => method.to_string(), "path" => path, "status" => status)
        .record(duration);

    response
}

/// Handler for GET /metrics — returns Prometheus text format.
pub async fn metrics_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Record queue depth gauge on each scrape
    if let Ok(counts) = notifico_db::repo::queue::count_by_status(&state.db).await {
        for (status, count) in &counts {
            metrics::gauge!("delivery_queue_depth", "status" => status.clone())
                .set(*count as f64);
        }
    }

    let handle = state.metrics_handle.as_ref();
    match handle {
        Some(h) => h.render(),
        None => String::from("# metrics not initialized\n"),
    }
}

/// Install the Prometheus exporter and return the render handle.
pub fn install_prometheus_recorder(
) -> metrics_exporter_prometheus::PrometheusHandle {
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    builder
        .install_recorder()
        .expect("Failed to install Prometheus recorder")
}
