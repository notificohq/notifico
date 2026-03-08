use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::error::CoreError;
use crate::pipeline::PipelineOutput;

use super::Middleware;

/// Post-render middleware that appends UTM query parameters to URLs in HTML.
pub struct UtmParamsMiddleware;

#[async_trait]
impl Middleware for UtmParamsMiddleware {
    fn name(&self) -> &str {
        "utm_params"
    }

    async fn post_render(
        &self,
        output: &mut PipelineOutput,
        config: &Value,
    ) -> Result<(), CoreError> {
        if let Value::Object(ref mut map) = output.rendered_body {
            if let Some(Value::String(html)) = map.get_mut("html") {
                let source = config
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("notifico");
                let medium = config
                    .get("medium")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&output.channel);
                let campaign = config
                    .get("campaign")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&output.event_name);

                let utm = format!(
                    "utm_source={}&utm_medium={}&utm_campaign={}",
                    source, medium, campaign
                );

                let re = Regex::new(r#"href="([^"]+)""#).unwrap();
                let new_html = re
                    .replace_all(html, |caps: &regex::Captures| {
                        let url = &caps[1];
                        if should_skip_url(url) {
                            return caps[0].to_string();
                        }
                        let separator = if url.contains('?') { "&" } else { "?" };
                        format!(r#"href="{}{}{}""#, url, separator, utm)
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
            event_name: "order.confirmed".into(),
            recipient_id: Uuid::now_v7(),
            channel: "email".into(),
            rendered_body,
            contact_value: "user@example.com".into(),
            idempotency_key: None,
            max_attempts: 3,
        }
    }

    #[tokio::test]
    async fn utm_appends_to_clean_urls() {
        let mw = UtmParamsMiddleware;
        let config = json!({});
        let mut output = make_output(json!({
            "html": r#"<a href="https://example.com/page">Link</a>"#
        }));

        mw.post_render(&mut output, &config).await.unwrap();

        let html = output.rendered_body["html"].as_str().unwrap();
        assert!(html.contains("utm_source=notifico"));
        assert!(html.contains("utm_medium=email"));
        assert!(html.contains("utm_campaign=order.confirmed"));
        assert!(html.contains("https://example.com/page?utm_source"));
    }

    #[tokio::test]
    async fn utm_appends_to_urls_with_query() {
        let mw = UtmParamsMiddleware;
        let config = json!({});
        let mut output = make_output(json!({
            "html": r#"<a href="https://example.com/page?id=42">Link</a>"#
        }));

        mw.post_render(&mut output, &config).await.unwrap();

        let html = output.rendered_body["html"].as_str().unwrap();
        assert!(html.contains("https://example.com/page?id=42&utm_source"));
    }

    #[tokio::test]
    async fn utm_skips_mailto() {
        let mw = UtmParamsMiddleware;
        let config = json!({});
        let mut output = make_output(json!({
            "html": r#"<a href="mailto:test@example.com">Email</a>"#
        }));

        mw.post_render(&mut output, &config).await.unwrap();

        let html = output.rendered_body["html"].as_str().unwrap();
        assert!(html.contains("mailto:test@example.com"));
        assert!(!html.contains("utm_source"));
    }

    #[tokio::test]
    async fn utm_noop_without_html() {
        let mw = UtmParamsMiddleware;
        let config = json!({});
        let mut output = make_output(json!({"subject": "Hello"}));
        let original = output.rendered_body.clone();

        mw.post_render(&mut output, &config).await.unwrap();

        assert_eq!(output.rendered_body, original);
    }
}
