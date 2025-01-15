use crate::entity;
use crate::entity::prelude::*;
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::{stream, StreamExt};
use notifico_core::error::EngineError;
use notifico_core::pipeline::event::RecipientSelector;
use notifico_core::recipient::RecipientController;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

pub struct RecipientDbSource {
    db: DatabaseConnection,
}

impl RecipientDbSource {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RecipientController for RecipientDbSource {
    async fn get_recipients(
        &self,
        project: Uuid,
        sel: Vec<RecipientSelector>,
    ) -> Result<BoxStream<notifico_core::recipient::Recipient>, EngineError> {
        let mut inlines = vec![];
        let mut ids = vec![];

        for selector in sel {
            match selector {
                RecipientSelector::Id(id) => ids.push(id),
                RecipientSelector::Recipient(inline) => inlines.push(inline),
            }
        }

        // Fetch recipient ids by group id
        let groups = GroupMembership::find()
            .filter(entity::group_membership::Column::GroupId.is_in(ids.clone()))
            .all(&self.db)
            .await?;
        ids.extend(groups.into_iter().map(|g| g.recipient_id));

        // Individual recipient IDs
        let recipients = Recipient::find()
            .find_with_related(Contact)
            .filter(entity::recipient::Column::Id.is_in(ids.clone()))
            .filter(entity::recipient::Column::ProjectId.eq(project))
            .all(&self.db)
            .await?;

        let mut core_recipients = vec![];
        for (recipient, contacts) in recipients {
            let mut core_recipient = notifico_core::recipient::Recipient {
                id: recipient.id,
                contacts: vec![],
            };
            for contact in contacts {
                core_recipient.contacts.push(contact.contact.parse()?);
            }
            core_recipients.push(core_recipient);
        }
        let individual_recipients = stream::iter(core_recipients);

        Ok(stream::iter(inlines).chain(individual_recipients).boxed())
    }
}
