use std::collections::HashMap;
use std::sync::Arc;

use crate::channel::ChannelId;
use crate::transport::{ContentSchema, CredentialSchema, Transport};

/// Registry of available transports. Populated at startup.
pub struct TransportRegistry {
    transports: HashMap<ChannelId, Arc<dyn Transport>>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Self {
            transports: HashMap::new(),
        }
    }

    /// Register a transport. Panics if channel_id is already registered.
    pub fn register(&mut self, transport: Arc<dyn Transport>) {
        let channel_id = transport.channel_id();
        if self.transports.contains_key(&channel_id) {
            panic!("Transport already registered for channel: {}", channel_id);
        }
        self.transports.insert(channel_id, transport);
    }

    /// Get a transport by channel ID.
    pub fn get(&self, channel_id: &ChannelId) -> Option<&Arc<dyn Transport>> {
        self.transports.get(channel_id)
    }

    /// List all registered channel IDs.
    pub fn channels(&self) -> Vec<ChannelId> {
        self.transports.keys().cloned().collect()
    }

    /// Get channel info for admin UI.
    pub fn channel_info(&self) -> Vec<ChannelInfo> {
        self.transports
            .values()
            .map(|t| ChannelInfo {
                channel_id: t.channel_id(),
                display_name: t.display_name().to_string(),
                content_schema: t.content_schema(),
                credential_schema: t.credential_schema(),
            })
            .collect()
    }
}

impl Default for TransportRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub channel_id: ChannelId,
    pub display_name: String,
    pub content_schema: ContentSchema,
    pub credential_schema: CredentialSchema,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CoreError;
    use crate::transport::*;
    use async_trait::async_trait;

    struct FakeTransport {
        id: ChannelId,
    }

    #[async_trait]
    impl Transport for FakeTransport {
        fn channel_id(&self) -> ChannelId {
            self.id.clone()
        }
        fn display_name(&self) -> &str {
            "Fake"
        }
        fn content_schema(&self) -> ContentSchema {
            ContentSchema { fields: vec![] }
        }
        fn credential_schema(&self) -> CredentialSchema {
            CredentialSchema { fields: vec![] }
        }
        async fn send(&self, _msg: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
            Ok(DeliveryResult::Delivered {
                provider_message_id: None,
            })
        }
    }

    #[test]
    fn register_and_get_transport() {
        let mut registry = TransportRegistry::new();
        let transport = Arc::new(FakeTransport {
            id: ChannelId::new("fake"),
        });
        registry.register(transport);

        assert!(registry.get(&ChannelId::new("fake")).is_some());
        assert!(registry.get(&ChannelId::new("missing")).is_none());
    }

    #[test]
    fn list_channels() {
        let mut registry = TransportRegistry::new();
        registry.register(Arc::new(FakeTransport {
            id: ChannelId::new("email"),
        }));
        registry.register(Arc::new(FakeTransport {
            id: ChannelId::new("sms"),
        }));

        let channels = registry.channels();
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&ChannelId::new("email")));
        assert!(channels.contains(&ChannelId::new("sms")));
    }

    #[test]
    fn channel_info() {
        let mut registry = TransportRegistry::new();
        registry.register(Arc::new(FakeTransport {
            id: ChannelId::new("email"),
        }));

        let info = registry.channel_info();
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].display_name, "Fake");
    }

    #[test]
    #[should_panic(expected = "Transport already registered")]
    fn duplicate_registration_panics() {
        let mut registry = TransportRegistry::new();
        registry.register(Arc::new(FakeTransport {
            id: ChannelId::new("email"),
        }));
        registry.register(Arc::new(FakeTransport {
            id: ChannelId::new("email"),
        }));
    }
}
