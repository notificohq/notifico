use async_trait::async_trait;
use serde_json::Value;

use crate::error::CoreError;
use crate::pipeline::PipelineOutput;

use super::Middleware;

/// Post-render middleware that generates a plain-text fallback from HTML
/// when a `text` field does not already exist.
pub struct PlaintextFallbackMiddleware;

#[async_trait]
impl Middleware for PlaintextFallbackMiddleware {
    fn name(&self) -> &str {
        "plaintext_fallback"
    }

    async fn post_render(
        &self,
        output: &mut PipelineOutput,
        config: &Value,
    ) -> Result<(), CoreError> {
        if let Value::Object(ref mut map) = output.rendered_body {
            // Only generate if html exists and text doesn't
            if map.contains_key("html") && !map.contains_key("text") {
                if let Some(Value::String(html)) = map.get("html") {
                    let width = config
                        .get("width")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(80) as usize;
                    if let Ok(text) = html2text::from_read(html.as_bytes(), width) {
                        map.insert("text".into(), Value::String(text));
                    }
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
    async fn plaintext_generates_from_html() {
        let mw = PlaintextFallbackMiddleware;
        let config = json!({});
        let mut output = make_output(json!({
            "html": "<h1>Hello</h1><p>World</p>"
        }));

        mw.post_render(&mut output, &config).await.unwrap();

        let text = output.rendered_body["text"].as_str().unwrap();
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[tokio::test]
    async fn plaintext_does_not_overwrite_existing() {
        let mw = PlaintextFallbackMiddleware;
        let config = json!({});
        let mut output = make_output(json!({
            "html": "<p>HTML version</p>",
            "text": "Custom text"
        }));

        mw.post_render(&mut output, &config).await.unwrap();

        assert_eq!(output.rendered_body["text"], "Custom text");
    }

    #[tokio::test]
    async fn plaintext_noop_without_html() {
        let mw = PlaintextFallbackMiddleware;
        let config = json!({});
        let mut output = make_output(json!({"subject": "Hello"}));
        let original = output.rendered_body.clone();

        mw.post_render(&mut output, &config).await.unwrap();

        assert_eq!(output.rendered_body, original);
    }
}
