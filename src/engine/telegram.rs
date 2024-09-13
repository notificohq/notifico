use crate::engine::templater::RenderedContext;
use crate::engine::{Engine, EngineError, EnginePlugin, PipelineContext};
use async_trait::async_trait;
use sea_orm::JsonValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::sync::Arc;
use teloxide::prelude::{ChatId, Requester};
use teloxide::types::Recipient;
use teloxide::Bot;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct TelegramBotCredentials {
    token: String,
}

#[derive(Default)]
pub struct TelegramPlugin {}

impl TelegramPlugin {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TelegramStep {
    #[serde(rename = "telegram.load_template")]
    LoadTemplate { template_id: Uuid },
    // #[serde(rename = "telegram.set_recipients")]
    // SetRecipients { telegram_id: Vec<i64> },
    #[serde(rename = "telegram.send")]
    Send { bot_token: String },
}

#[derive(Default, Serialize, Deserialize)]
struct TelegramContext {
    template_id: Uuid,
}

#[async_trait]
impl EnginePlugin for TelegramPlugin {
    async fn execute_step(
        &self,
        engine: &Engine,
        context: &mut PipelineContext,
        step: Value,
    ) -> Result<(), EngineError> {
        let telegram_context = context
            .plugin_contexts
            .entry("telegram".into())
            .or_insert(Value::Object(Default::default()));

        let mut plugin_context: TelegramContext =
            serde_json::from_value(telegram_context.clone()).unwrap();
        let telegram_step: TelegramStep = serde_json::from_value(step).unwrap();

        match telegram_step {
            TelegramStep::LoadTemplate { template_id } => {
                plugin_context.template_id = template_id;
                context.plugin_contexts.insert(
                    "telegram".into(),
                    serde_json::to_value(plugin_context).unwrap(),
                );
            }
            TelegramStep::Send { bot_token } => {
                let bot = Bot::new(bot_token);

                for recipient in context.recipients.iter() {
                    let rendered_template = engine
                        .get_templater()
                        .render(
                            "telegram",
                            plugin_context.template_id,
                            context.event_context.0.clone(),
                        )
                        .await
                        .unwrap();

                    let rendered_template: TelegramTemplate = rendered_template.try_into().unwrap();

                    bot.send_message(
                        Recipient::Id(ChatId(recipient.telegram_id)),
                        rendered_template.clone().body,
                    )
                    .await
                    .unwrap();
                }
            }
        }

        Ok(())
    }

    fn step_type(&self) -> Cow<'static, str> {
        "telegram".into()
    }
}

#[derive(Deserialize, Clone)]
pub struct TelegramTemplate {
    pub body: String,
}

impl TryFrom<RenderedContext> for TelegramTemplate {
    type Error = ();

    fn try_from(value: RenderedContext) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0)).map_err(|_| ())
    }
}
