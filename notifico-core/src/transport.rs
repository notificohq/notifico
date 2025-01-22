use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::Arc;

pub trait Transport {
    fn name(&self) -> Cow<'static, str>;
    fn send_step(&self) -> Cow<'static, str>;
    fn supported_channels(&self) -> Vec<Cow<'static, str>>;
}

#[derive(Default)]
pub struct TransportRegistry {
    transports: HashMap<Cow<'static, str>, Cow<'static, str>>,
    channels: HashSet<Cow<'static, str>>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register(&mut self, transport: Arc<dyn Transport>) {
        self.transports
            .insert(transport.name(), transport.send_step());
        self.channels.extend(transport.supported_channels());
    }

    pub fn get_step(&self, name: &str) -> Option<&str> {
        self.transports.get(name).map(|s| s.deref())
    }

    pub fn supported_channels(&self) -> &HashSet<Cow<'static, str>> {
        &self.channels
    }
}
