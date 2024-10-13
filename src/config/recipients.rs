use crate::config::Config;
use async_trait::async_trait;
use notifico_core::recipient::{Recipient, RecipientDirectory};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Default)]
pub struct SimpleRecipientDirectory {
    directory: HashMap<Uuid, HashMap<String, Recipient>>,
}

impl SimpleRecipientDirectory {
    pub fn from_config(config: &Config) -> Self {
        let mut slf = Self::default();
        for project in config.projects.iter() {
            slf.add_project(project.id, project.recipients.clone());
        }
        slf
    }

    fn add_project(&mut self, project: Uuid, recipients: Vec<Recipient>) {
        let recipients: HashMap<String, Recipient> =
            recipients.into_iter().map(|r| (r.id.clone(), r)).collect();
        if self.directory.insert(project, recipients).is_some() {
            panic!("Project already exists: {}", project);
        }
    }
}

#[async_trait]
impl RecipientDirectory for SimpleRecipientDirectory {
    async fn get_recipient(&self, project: Uuid, id: &str) -> Option<Recipient> {
        let recipients = self.directory.get(&project)?;
        recipients.get(id).cloned()
    }
}
