use async_trait::async_trait;
use serde_json::Value;

use crate::error::CoreError;
use crate::pipeline::PipelineOutput;

use super::Middleware;

/// Post-render middleware that adds unsubscribe URL metadata and an HTML
/// unsubscribe link to rendered email bodies.
pub struct UnsubscribeLinkMiddleware;

#[async_trait]
impl Middleware for UnsubscribeLinkMiddleware {
    fn name(&self) -> &str {
        "unsubscribe_link"
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

        let unsub_url = format!(
            "{}/api/v1/public/unsubscribe?token={}",
            base_url, output.recipient_id
        );

        if let Value::Object(ref mut map) = output.rendered_body {
            map.insert(
                "_unsubscribe_url".into(),
                Value::String(unsub_url.clone()),
            );
            map.insert(
                "_list_unsubscribe".into(),
                Value::String(format!("<{}>", unsub_url)),
            );

            // Append unsubscribe link to HTML if present
            if let Some(Value::String(html)) = map.get_mut("html") {
                let link = format!(
                    r#"<p style="font-size:12px;color:#666;text-align:center;"><a href="{}">Unsubscribe</a></p>"#,
                    unsub_url
                );
                if let Some(pos) = html.rfind("</body>") {
                    html.insert_str(pos, &link);
                } else {
                    html.push_str(&link);
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
    async fn unsubscribe_adds_url_to_rendered_body() {
        let mw = UnsubscribeLinkMiddleware;
        let config = json!({"base_url": "https://example.com"});
        let mut output = make_output(json!({"subject": "Hello"}));
        let recipient_id = output.recipient_id;

        mw.post_render(&mut output, &config).await.unwrap();

        let map = output.rendered_body.as_object().unwrap();
        let expected_url = format!(
            "https://example.com/api/v1/public/unsubscribe?token={}",
            recipient_id
        );
        assert_eq!(map["_unsubscribe_url"], expected_url);
        assert_eq!(map["_list_unsubscribe"], format!("<{}>", expected_url));
    }

    #[tokio::test]
    async fn unsubscribe_appends_link_to_html() {
        let mw = UnsubscribeLinkMiddleware;
        let config = json!({"base_url": "https://example.com"});
        let mut output = make_output(json!({
            "html": "<html><body><p>Hello</p></body></html>"
        }));

        mw.post_render(&mut output, &config).await.unwrap();

        let html = output.rendered_body["html"].as_str().unwrap();
        assert!(html.contains("Unsubscribe</a></p></body>"));
    }

    #[tokio::test]
    async fn unsubscribe_noop_without_base_url() {
        let mw = UnsubscribeLinkMiddleware;
        let config = json!({});
        let mut output = make_output(json!({"html": "<p>Hello</p>"}));
        let original = output.rendered_body.clone();

        mw.post_render(&mut output, &config).await.unwrap();

        assert_eq!(output.rendered_body, original);
    }
}
