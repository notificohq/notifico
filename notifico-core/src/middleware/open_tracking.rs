use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::Value;

use crate::error::CoreError;
use crate::pipeline::PipelineOutput;

use super::Middleware;

/// Post-render middleware that appends a 1x1 tracking pixel to HTML content.
pub struct OpenTrackingMiddleware;

#[async_trait]
impl Middleware for OpenTrackingMiddleware {
    fn name(&self) -> &str {
        "open_tracking"
    }

    async fn post_render(
        &self,
        output: &mut PipelineOutput,
        config: &Value,
    ) -> Result<(), CoreError> {
        let base_url = config
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if base_url.is_empty() {
            return Ok(());
        }

        if let Value::Object(ref mut map) = output.rendered_body {
            if let Some(Value::String(html)) = map.get_mut("html") {
                let token = URL_SAFE_NO_PAD.encode(output.id.to_string());
                let pixel = format!(
                    r#"<img src="{}/t/open/{}" width="1" height="1" style="display:none" alt="" />"#,
                    base_url, token
                );
                if let Some(pos) = html.rfind("</body>") {
                    html.insert_str(pos, &pixel);
                } else {
                    html.push_str(&pixel);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn make_output(rendered_body: Value) -> PipelineOutput {
        PipelineOutput {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            event_name: "test.event".into(),
            recipient_id: Uuid::now_v7(),
            channel: "email".into(),
            rendered_body,
            contact_value: "user@example.com".into(),
            idempotency_key: None,
            max_attempts: 3,
        }
    }

    #[tokio::test]
    async fn open_tracking_appends_pixel() {
        let mw = OpenTrackingMiddleware;
        let config = json!({"base_url": "https://track.example.com"});
        let mut output = make_output(json!({
            "html": "<html><body><p>Hello</p></body></html>"
        }));

        mw.post_render(&mut output, &config).await.unwrap();

        let html = output.rendered_body["html"].as_str().unwrap();
        assert!(html.contains("/t/open/"));
        assert!(html.contains(r#"width="1" height="1""#));
        assert!(html.contains("</body>"));
    }

    #[tokio::test]
    async fn open_tracking_no_html_is_noop() {
        let mw = OpenTrackingMiddleware;
        let config = json!({"base_url": "https://track.example.com"});
        let mut output = make_output(json!({"subject": "Hello"}));
        let original = output.rendered_body.clone();

        mw.post_render(&mut output, &config).await.unwrap();

        assert_eq!(output.rendered_body, original);
    }

    #[tokio::test]
    async fn open_tracking_noop_without_base_url() {
        let mw = OpenTrackingMiddleware;
        let config = json!({});
        let mut output = make_output(json!({"html": "<p>Hello</p>"}));
        let original = output.rendered_body.clone();

        mw.post_render(&mut output, &config).await.unwrap();

        assert_eq!(output.rendered_body, original);
    }
}
