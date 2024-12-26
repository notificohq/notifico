use crate::entity::contact::Entity as Contact;
use crate::entity::recipient::Entity as Recipient;
use crate::RecipientDbController;
use futures::stream::BoxStream;
use futures::{stream, StreamExt};

// #[async_trait]
// impl RecipientController for SubscriptionController {
//     async fn get_recipients(
//         &self,
//         project: Uuid,
//         sel: Vec<RecipientSelector>,
//     ) -> Result<BoxStream<notifico_core::recipient::Recipient>, EngineError> {
//         let mut inlines = vec![];
//         let mut ids = vec![];
//
//         for selector in sel {
//             match selector {
//                 RecipientSelector::Id(id) => ids.push(id),
//                 RecipientSelector::Recipient(inline) => inlines.push(inline),
//             }
//         }
//
//         // Individual recipient IDs
//         let recipients = Recipient::find()
//             .find_with_related(Contact)
//             .filter(entity::recipient::Column::Id.is_in(ids.clone()))
//             .filter(entity::recipient::Column::ProjectId.eq(project))
//             .all(&self.db)
//             .await?;
//
//         let mut core_recipients = vec![];
//         for (recipient, contacts) in recipients {
//             let mut core_recipient = notifico_core::recipient::Recipient {
//                 id: recipient.id,
//                 contacts: vec![],
//             };
//             for contact in contacts {
//                 core_recipient.contacts.push(contact.contact.parse()?);
//             }
//             core_recipients.push(core_recipient);
//         }
//         let individual_recipients = stream::iter(core_recipients);
//
//         // Recipient Group IDs
//         // let group_
//
//         Ok(stream::iter(inlines).chain(individual_recipients).boxed())
//     }
// }
