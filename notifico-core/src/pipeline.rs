use serde_json::Value;
use uuid::Uuid;

/// Input for the pipeline: one recipient + one pipeline rule match.
#[derive(Debug, Clone)]
pub struct PipelineInput {
    pub project_id: Uuid,
    pub event_name: String,
    pub recipient_id: Uuid,
    pub recipient_locale: String,
    pub channel: String,
    pub contact_value: String,
    pub template_body: Value,
    pub context_data: Value,
    pub idempotency_key: Option<String>,
    pub max_attempts: u32,
}

/// Output of the pipeline: a delivery task ready for enqueuing.
#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_name: String,
    pub recipient_id: Uuid,
    pub channel: String,
    pub rendered_body: Value,
    pub contact_value: String,
    pub idempotency_key: Option<String>,
    pub max_attempts: u32,
}

/// Execute the rendering pipeline for one recipient + one channel.
///
/// Steps:
/// 1. Render template body fields via minijinja (notifico-template)
/// 2. Return PipelineOutput ready for enqueuing
pub fn execute_pipeline(input: PipelineInput) -> Result<PipelineOutput, crate::error::CoreError> {
    let rendered = notifico_template::render_body(&input.template_body, &input.context_data)
        .map_err(|e| crate::error::CoreError::TemplateRender(e.to_string()))?;

    Ok(PipelineOutput {
        id: Uuid::now_v7(),
        project_id: input.project_id,
        event_name: input.event_name,
        recipient_id: input.recipient_id,
        channel: input.channel,
        rendered_body: Value::Object(rendered),
        contact_value: input.contact_value,
        idempotency_key: input.idempotency_key,
        max_attempts: input.max_attempts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_input(body: Value, data: Value) -> PipelineInput {
        PipelineInput {
            project_id: Uuid::now_v7(),
            event_name: "order.confirmed".into(),
            recipient_id: Uuid::now_v7(),
            recipient_locale: "en".into(),
            channel: "email".into(),
            contact_value: "user@example.com".into(),
            template_body: body,
            context_data: data,
            idempotency_key: None,
            max_attempts: 5,
        }
    }

    #[test]
    fn execute_pipeline_renders_template() {
        let input = make_input(
            json!({"subject": "Order #{{ order_id }}", "text": "Hello {{ name }}"}),
            json!({"order_id": 42, "name": "Alice"}),
        );
        let output = execute_pipeline(input).unwrap();
        assert_eq!(output.rendered_body["subject"], "Order #42");
        assert_eq!(output.rendered_body["text"], "Hello Alice");
        assert_eq!(output.channel, "email");
    }

    #[test]
    fn execute_pipeline_preserves_metadata() {
        let input = make_input(json!({"text": "Hi {{ name }}"}), json!({"name": "Bob"}));
        let project_id = input.project_id;
        let recipient_id = input.recipient_id;
        let output = execute_pipeline(input).unwrap();
        assert_eq!(output.project_id, project_id);
        assert_eq!(output.recipient_id, recipient_id);
        assert_eq!(output.event_name, "order.confirmed");
    }

    #[test]
    fn execute_pipeline_with_idempotency_key() {
        let mut input = make_input(json!({"text": "Hello"}), json!({}));
        input.idempotency_key = Some("key-abc".into());
        let output = execute_pipeline(input).unwrap();
        assert_eq!(output.idempotency_key, Some("key-abc".into()));
    }

    #[test]
    fn execute_pipeline_passthrough_non_string() {
        let input = make_input(
            json!({
                "text": "Hello {{ name }}",
                "buttons": [{"label": "View", "url": "https://example.com"}]
            }),
            json!({"name": "Carol"}),
        );
        let output = execute_pipeline(input).unwrap();
        assert_eq!(output.rendered_body["text"], "Hello Carol");
        assert_eq!(
            output.rendered_body["buttons"],
            json!([{"label": "View", "url": "https://example.com"}])
        );
    }

    #[test]
    fn execute_pipeline_invalid_body_returns_error() {
        let input = make_input(json!("not an object"), json!({}));
        let result = execute_pipeline(input);
        assert!(result.is_err());
    }
}
