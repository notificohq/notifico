use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

pub trait Transport {
    fn name(&self) -> Cow<'static, str>;
    fn send_step(&self) -> Cow<'static, str>;
}

#[derive(Default)]
pub struct TransportRegistry {
    transports: HashMap<String, String>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn register(&mut self, transport: Arc<dyn Transport>) {
        self.transports.insert(
            transport.name().to_string(),
            transport.send_step().to_string(),
        );
    }

    pub fn get_step(&self, name: &str) -> Option<&str> {
        self.transports.get(name).map(|s| s.as_str())
    }
}
