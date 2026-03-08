use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use regex::Regex;
use serde_json::Value;

use crate::error::CoreError;
use crate::pipeline::PipelineOutput;

use super::Middleware;

/// Post-render middleware that rewrites URLs in HTML content with click-tracking
/// redirect URLs.
pub struct ClickTrackingMiddleware;

#[async_trait]
impl Middleware for ClickTrackingMiddleware {
    fn name(&self) -> &str {
        "click_tracking"
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
                let delivery_id = output.id;
                let base = base_url.to_string();
                let re = Regex::new(r#"href="([^"]+)""#).unwrap();
                let new_html = re
                    .replace_all(html, |caps: &regex::Captures| {
                        let url = &caps[1];
                        if should_skip_url(url) {
                            return caps[0].to_string();
                        }
                        let token =
                            URL_SAFE_NO_PAD.encode(format!("{}:{}", delivery_id, url));
                        format!(r#"href="{}/t/click/{}""#, base, token)
                    })
                    .to_string();
                *html = new_html;
            }
        }
        Ok(())
    }
}

fn should_skip_url(url: &str) -> bool {
    url.starts_with("mailto:")
        || url.starts_with("tel:")
        || url.starts_with('#')
        || url.starts_with("javascript:")
        || url.contains("/t/click/")
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
    async fn click_tracking_rewrites_urls() {
        let mw = ClickTrackingMiddleware;
        let config = json!({"base_url": "https://track.example.com"});
        let mut output = make_output(json!({
            "html": r#"<a href="https://example.com/page">Link</a>"#
        }));

        mw.post_render(&mut output, &config).await.unwrap();

        let html = output.rendered_body["html"].as_str().unwrap();
        assert!(html.contains("https://track.example.com/t/click/"));
        assert!(!html.contains("https://example.com/page"));
    }

    #[tokio::test]
    async fn click_tracking_skips_mailto() {
        let mw = ClickTrackingMiddleware;
        let config = json!({"base_url": "https://track.example.com"});
        let mut output = make_output(json!({
            "html": "<a href=\"mailto:test@example.com\">Email</a> <a href=\"tel:+1234\">Call</a> <a href=\"#top\">Top</a>"
        }));

        mw.post_render(&mut output, &config).await.unwrap();

        let html = output.rendered_body["html"].as_str().unwrap();
        assert!(html.contains("mailto:test@example.com"));
        assert!(html.contains("tel:+1234"));
        assert!(html.contains("#top"));
    }

    #[tokio::test]
    async fn click_tracking_noop_without_base_url() {
        let mw = ClickTrackingMiddleware;
        let config = json!({});
        let mut output = make_output(json!({
            "html": r#"<a href="https://example.com">Link</a>"#
        }));
        let original = output.rendered_body.clone();

        mw.post_render(&mut output, &config).await.unwrap();

        assert_eq!(output.rendered_body, original);
    }
}
