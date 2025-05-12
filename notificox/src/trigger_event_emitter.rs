use tokio::sync::broadcast;

#[derive(Clone)]
pub struct TriggerEventEmitter {
    sender: broadcast::Sender<u32>,
}

impl TriggerEventEmitter {
    pub(crate) fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<u32> {
        self.sender.subscribe()
    }

    pub fn emit(&self, token: u32) {
        self.sender.send(token).unwrap();
    }
}
