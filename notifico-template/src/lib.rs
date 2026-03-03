use minijinja::Environment;
use serde_json::Value;
use thiserror::Error;

/// Errors that can occur during template rendering.
#[derive(Debug, Error)]
pub enum TemplateError {
    /// A MiniJinja rendering error.
    #[error("Template render error: {0}")]
    Render(#[from] minijinja::Error),

    /// The requested template was not found.
    #[error("Template not found: {0}")]
    NotFound(String),

    /// The template body is not a valid JSON object.
    #[error("Invalid template body: {0}")]
    InvalidBody(String),
}

/// Render a single Jinja2 template string with the given context.
///
/// The `context` value is passed directly to MiniJinja as the template context,
/// so it can be an object, array, or any JSON value that the template expects.
pub fn render_string(template: &str, context: &Value) -> Result<String, TemplateError> {
    let env = Environment::new();
    let result = env.render_str(template, context)?;
    Ok(result)
}

/// Render all string fields in a JSON object body, passing non-string values through unchanged.
///
/// Each string value in the top-level object is treated as a Jinja2 template and rendered
/// with the provided context. Non-string values (arrays, numbers, booleans, nested objects, null)
/// are included in the output without modification.
///
/// Returns an error if `body` is not a JSON object.
pub fn render_body(
    body: &Value,
    context: &Value,
) -> Result<serde_json::Map<String, Value>, TemplateError> {
    let obj = body
        .as_object()
        .ok_or_else(|| TemplateError::InvalidBody("body must be a JSON object".to_string()))?;

    let env = Environment::new();
    let mut result = serde_json::Map::new();

    for (key, value) in obj {
        match value {
            Value::String(template_str) => {
                let rendered = env.render_str(template_str, context)?;
                result.insert(key.clone(), Value::String(rendered));
            }
            other => {
                result.insert(key.clone(), other.clone());
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_simple_string() {
        let template = "Hello {{ name }}!";
        let context = json!({"name": "World"});
        let result = render_string(template, &context).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn render_string_with_filter() {
        let template = "{{ name | upper }}";
        let context = json!({"name": "alice"});
        let result = render_string(template, &context).unwrap();
        assert_eq!(result, "ALICE");
    }

    #[test]
    fn render_string_with_loop() {
        let template = "{% for item in items %}{{ item }},{% endfor %}";
        let context = json!({"items": ["a", "b", "c"]});
        let result = render_string(template, &context).unwrap();
        assert_eq!(result, "a,b,c,");
    }

    #[test]
    fn render_body_multiple_fields() {
        let body = json!({
            "subject": "Hello {{ name }}",
            "text": "Dear {{ name }}, welcome!"
        });
        let context = json!({"name": "Alice"});
        let result = render_body(&body, &context).unwrap();
        assert_eq!(result["subject"], "Hello Alice");
        assert_eq!(result["text"], "Dear Alice, welcome!");
    }

    #[test]
    fn render_body_non_string_passthrough() {
        let body = json!({
            "subject": "Hello {{ name }}",
            "buttons": [{"label": "Click me", "url": "https://example.com"}]
        });
        let context = json!({"name": "Bob"});
        let result = render_body(&body, &context).unwrap();
        assert_eq!(result["subject"], "Hello Bob");
        assert_eq!(
            result["buttons"],
            json!([{"label": "Click me", "url": "https://example.com"}])
        );
    }

    #[test]
    fn render_body_invalid_non_object() {
        let body = json!("not an object");
        let context = json!({"name": "Test"});
        let result = render_body(&body, &context);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TemplateError::InvalidBody(_)));
    }
}
