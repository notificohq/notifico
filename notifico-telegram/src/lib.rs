use async_trait::async_trait;
use contact::TelegramContact;
use notifico_core::credentials::Credentials;
use notifico_core::engine::plugin::EnginePlugin;
use notifico_core::engine::PipelineContext;
use notifico_core::error::EngineError;
use notifico_core::pipeline::SerializedStep;
use notifico_core::templater::{RenderResponse, Templater};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::sync::Arc;
use step::{CredentialSelector, TelegramStep};
use teloxide::prelude::Requester;
use teloxide::Bot;

const CHANNEL_NAME: &'static str = "telegram";

mod contact;
mod step;

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramBotCredentials {
    token: String,
}

impl TryFrom<Value> for TelegramBotCredentials {
    type Error = EngineError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value).map_err(|_| EngineError::InvalidCredentialFormat)
    }
}

pub struct TelegramPlugin {
    templater: Arc<dyn Templater>,
    credentials: Arc<dyn Credentials>,
}

impl TelegramPlugin {
    pub fn new(templater: Arc<dyn Templater>, credentials: Arc<dyn Credentials>) -> Self {
        Self {
            templater,
            credentials,
        }
    }
}

#[async_trait]
impl EnginePlugin for TelegramPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<(), EngineError> {
        let telegram_step: TelegramStep = step.clone().try_into().unwrap();

        match telegram_step {
            TelegramStep::LoadTemplate { template_id } => {
                if context.recipient.is_none() {
                    return Err(EngineError::RecipientNotSet);
                };

                let rendered_template = self
                    .templater
                    .render("telegram", template_id, context.event_context.0.clone())
                    .await?;

                let rendered_template: TelegramContent = rendered_template.try_into().unwrap();

                context.plugin_contexts.insert(
                    "telegram.content".into(),
                    serde_json::to_value(rendered_template)
                        .map_err(|_| EngineError::TemplateRenderingError)?,
                );
            }
            TelegramStep::Send(cred_selector) => {
                let Some(recipient) = context.recipient.clone() else {
                    return Err(EngineError::RecipientNotSet);
                };

                let content = context
                    .plugin_contexts
                    .get("telegram.content")
                    .ok_or(EngineError::TemplateNotSet)?;
                let content: TelegramContent = serde_json::from_value(content.clone())
                    .map_err(|e| EngineError::InternalError(e.into()))?;

                let contact: TelegramContact = recipient
                    .get_primary_contact("telegram")?
                    .clone()
                    .try_into()?;

                // Send
                let tgcred: TelegramBotCredentials = match cred_selector {
                    CredentialSelector::BotName { bot_name } => self
                        .credentials
                        .get_credential(context.project_id, "telegram_token", &bot_name)?
                        .try_into()?,
                };

                let bot = Bot::new(tgcred.token);
                bot.send_message(contact.into_recipient(), content.body)
                    .await
                    .unwrap();
            }
        }

        Ok(())
    }

    fn step_namespace(&self) -> Cow<'static, str> {
        "telegram".into()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TelegramContent {
    pub body: String,
}

impl TryFrom<RenderResponse> for TelegramContent {
    type Error = ();

    fn try_from(value: RenderResponse) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0)).map_err(|_| ())
    }
}
