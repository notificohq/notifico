pub mod click_tracking;
pub mod open_tracking;
pub mod plaintext_fallback;
pub mod unsubscribe_link;
pub mod utm_params;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::CoreError;
use crate::pipeline::{PipelineInput, PipelineOutput};
use crate::transport::{DeliveryResult, RenderedMessage};

/// Hook point in the pipeline where middleware runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPoint {
    PreRender,
    PostRender,
    PreSend,
    PostSend,
}

/// Pipeline middleware trait. All hooks have default no-op implementations.
/// `config` is per-rule middleware configuration from the database.
#[async_trait]
pub trait Middleware: Send + Sync {
    fn name(&self) -> &str;

    async fn pre_render(
        &self,
        _input: &mut PipelineInput,
        _config: &Value,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    async fn post_render(
        &self,
        _output: &mut PipelineOutput,
        _config: &Value,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    async fn pre_send(
        &self,
        _message: &mut RenderedMessage,
        _config: &Value,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    async fn post_send(
        &self,
        _message: &RenderedMessage,
        _result: &DeliveryResult,
        _config: &Value,
    ) -> Result<(), CoreError> {
        Ok(())
    }
}

/// Registry of available middleware implementations.
pub struct MiddlewareRegistry {
    middleware: std::collections::HashMap<String, std::sync::Arc<dyn Middleware>>,
}

impl MiddlewareRegistry {
    pub fn new() -> Self {
        Self {
            middleware: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, mw: std::sync::Arc<dyn Middleware>) {
        let name = mw.name().to_string();
        self.middleware.insert(name, mw);
    }

    pub fn get(&self, name: &str) -> Option<&std::sync::Arc<dyn Middleware>> {
        self.middleware.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.middleware.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for MiddlewareRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct NoopMiddleware;

    #[async_trait]
    impl Middleware for NoopMiddleware {
        fn name(&self) -> &str {
            "noop"
        }
    }

    #[test]
    fn register_and_lookup() {
        let mut registry = MiddlewareRegistry::new();
        let mw: Arc<dyn Middleware> = Arc::new(NoopMiddleware);
        registry.register(mw);

        let found = registry.get("noop");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "noop");

        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn list_middleware() {
        let mut registry = MiddlewareRegistry::new();
        registry.register(Arc::new(NoopMiddleware));

        let names = registry.list();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"noop"));
    }

    #[tokio::test]
    async fn default_hooks_are_noop() {
        let mw = NoopMiddleware;
        let config = serde_json::json!({});

        let mut input = crate::pipeline::PipelineInput {
            project_id: uuid::Uuid::now_v7(),
            event_name: "test.event".into(),
            recipient_id: uuid::Uuid::now_v7(),
            recipient_locale: "en".into(),
            channel: "email".into(),
            contact_value: "user@example.com".into(),
            template_body: serde_json::json!({"text": "hello"}),
            context_data: serde_json::json!({}),
            idempotency_key: None,
            max_attempts: 3,
        };

        let result = mw.pre_render(&mut input, &config).await;
        assert!(result.is_ok());
    }
}
