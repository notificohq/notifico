mod context;
mod step;

use crate::controllers::subscription::SubscriptionDbController;
use crate::plugin::context::EMAIL_LIST_UNSUBSCRIBE;
use crate::plugin::step::{Step, STEPS};
use jsonwebtoken::{EncodingKey, Header};
use migration::async_trait::async_trait;
use notifico_core::engine::{EnginePlugin, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::http::auth::Claims;
use notifico_core::step::SerializedStep;
use serde_json::Value;
use std::borrow::Cow;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;
use uuid::Uuid;

pub struct SubscriptionPlugin {
    controller: Arc<SubscriptionDbController>,
    secret_key: Vec<u8>,
    public_url: Option<Url>,
}

impl SubscriptionPlugin {
    pub fn new(
        controller: Arc<SubscriptionDbController>,
        secret_key: Vec<u8>,
        public_url: Option<Url>,
    ) -> Self {
        Self {
            controller,
            secret_key,
            public_url,
        }
    }
}

#[async_trait]
impl EnginePlugin for SubscriptionPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let Some(recipient) = &context.recipient else {
            return Err(EngineError::RecipientNotSet);
        };

        let step: Step = step.convert_step()?;

        match step {
            Step::Check { channel } => {
                if self
                    .controller
                    .is_subscribed(recipient.id, &context.event_name, &channel)
                    .await
                {
                    Ok(StepOutput::Continue)
                } else {
                    Ok(StepOutput::Interrupt)
                }
            }
            Step::ListUnsubscribe { .. } => {
                let Some(public_url) = self.public_url.clone() else {
                    return Err(EngineError::InvalidConfiguration(
                        "NOTIFICO_PUBLIC_URL is not set".to_string(),
                    ));
                };

                context.plugin_contexts.insert(
                    EMAIL_LIST_UNSUBSCRIBE.into(),
                    Value::String(format!(
                        "<{}>",
                        create_self_unsubscribe_url(
                            self.secret_key.clone(),
                            public_url,
                            &context.event_name,
                            recipient.id,
                        )
                    )),
                );
                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}

// Implements one-click List-Unsubscribe style URL generation
pub fn create_self_unsubscribe_url(
    key: Vec<u8>,
    subscriber_url: Url,
    event: &str,
    recipient_id: Uuid,
) -> Url {
    let claims = Claims::ListUnsubscribe {
        recipient_id,
        event: event.to_string(),
        exp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 60 * 60 * 24 * 30, // TODO: Move this into a configuration option
    };

    let token =
        jsonwebtoken::encode(&Header::default(), &claims, &EncodingKey::from_secret(&key)).unwrap();

    //TODO: Optimize URL creation to avoid format machinery
    subscriber_url
        .join(&format!(
            "api/recipient/v1/list_unsubscribe?token={}&event={}",
            token, event
        ))
        .unwrap()
}
