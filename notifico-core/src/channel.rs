use serde::{Deserialize, Serialize};

/// Channel identifier. String-based for extensibility (native + future WASM plugins).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(String);

impl ChannelId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_id_creation_and_display() {
        let ch = ChannelId::new("email");
        assert_eq!(ch.as_str(), "email");
        assert_eq!(ch.to_string(), "email");
    }

    #[test]
    fn channel_id_equality() {
        let a = ChannelId::new("sms");
        let b = ChannelId::new("sms");
        let c = ChannelId::new("email");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn channel_id_serialization() {
        let ch = ChannelId::new("telegram");
        let json = serde_json::to_string(&ch).unwrap();
        assert_eq!(json, "\"telegram\"");
        let deserialized: ChannelId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ch);
    }

    #[test]
    fn channel_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ChannelId::new("email"));
        set.insert(ChannelId::new("email"));
        set.insert(ChannelId::new("sms"));
        assert_eq!(set.len(), 2);
    }
}
