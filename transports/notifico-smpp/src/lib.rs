mod credentials;
mod step;

use crate::credentials::SmppServerCredentials;
use crate::step::{Step, STEPS};
use async_trait::async_trait;
use futures_util::sink::SinkExt;
use futures_util::StreamExt;
use notifico_core::credentials::CredentialStorage;
use notifico_core::engine::{EnginePlugin, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::recipient::PhoneContact;
use notifico_core::step::SerializedStep;
use notifico_core::templater::RenderedTemplate;
use notifico_core::transport::Transport;
use rusmpp::commands::tlvs::tlv::message_submission_request::MessageSubmissionRequestTLVValue;
use rusmpp::commands::types::{
    DataCoding, EsmClass, InterfaceVersion, Npi, RegisteredDelivery, ServiceType, Ton,
};
use rusmpp::pdu::{Bind, SubmitSm};
use rusmpp::types::{AnyOctetString, COctetString};
use rusmpp::{
    codec::command_codec::CommandCodec,
    commands::{
        command::Command,
        pdu::Pdu,
        types::{command_id::CommandId, command_status::CommandStatus},
    },
    TLVTag,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::debug;

pub struct SmppPlugin {
    credentials: Arc<dyn CredentialStorage>,
}

impl SmppPlugin {
    pub fn new(credentials: Arc<dyn CredentialStorage>) -> Self {
        Self { credentials }
    }
}

#[async_trait]
impl EnginePlugin for SmppPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.convert_step()?;

        match step {
            Step::Send { credential } => {
                let credential: SmppServerCredentials = self
                    .credentials
                    .resolve(context.project_id, credential)
                    .await?;

                let stream = TcpStream::connect((credential.host.clone(), credential.port))
                    .await
                    .unwrap();

                let (reader, writer) = stream.into_split();
                let mut framed_read = FramedRead::new(reader, CommandCodec {});
                let mut framed_write = FramedWrite::new(writer, CommandCodec {});

                // Build commands. Omitted values will be set to default.
                let bind_transceiver_command = Command::new(
                    CommandStatus::EsmeRok,
                    1,
                    Bind::builder()
                        .system_id(COctetString::from_str(&credential.username).unwrap())
                        .password(COctetString::from_str(&credential.password).unwrap())
                        .system_type(COctetString::empty())
                        .interface_version(InterfaceVersion::Smpp5_0)
                        .addr_ton(Ton::Unknown)
                        .addr_npi(Npi::Unknown)
                        .address_range(COctetString::empty())
                        .build()
                        .into_bind_transceiver(),
                );

                // Send commands.
                framed_write.send(&bind_transceiver_command).await.unwrap();

                // Wait for responses.
                while let Some(Ok(command)) = framed_read.next().await {
                    if let Some(Pdu::BindTransceiverResp(_)) = command.pdu() {
                        debug!("BindTransceiverResp received.");

                        if let CommandStatus::EsmeRok = command.command_status {
                            debug!("Successful bind.");
                            break;
                        }
                    }
                }

                let contact: Vec<PhoneContact> = context.get_recipient()?.get_contacts();

                for contact in contact {
                    for message in context.messages.iter().cloned() {
                        let rendered: SmsContent = message.content.try_into().unwrap();

                        let payload: Vec<u8> = rendered
                            .body
                            .encode_utf16()
                            .flat_map(|c| c.to_be_bytes())
                            .collect();

                        let submit_sm_command = Command::new(
                            CommandStatus::EsmeRok,
                            2,
                            SubmitSm::builder()
                                .serivce_type(ServiceType::default())
                                .source_addr_ton(Ton::Unknown)
                                .source_addr_npi(Npi::Unknown)
                                .source_addr(
                                    COctetString::from_str(&rendered.source_address).unwrap(),
                                )
                                .destination_addr(COctetString::from_str(contact.msisdn()).unwrap())
                                .esm_class(EsmClass::default())
                                .registered_delivery(RegisteredDelivery::request_all())
                                .data_coding(DataCoding::Ucs2)
                                .push_tlv(
                                    MessageSubmissionRequestTLVValue::MessagePayload(
                                        AnyOctetString::new(&payload),
                                    )
                                    .into(),
                                )
                                .build()
                                .into_submit_sm(),
                        );

                        framed_write.send(&submit_sm_command).await.unwrap();

                        'outer: while let Some(Ok(command)) = framed_read.next().await {
                            match command.pdu() {
                                Some(Pdu::SubmitSmResp(_)) => {
                                    debug!("SubmitSmResp received.");

                                    if let CommandStatus::EsmeRok = command.command_status {
                                        debug!("Successful submit.");
                                    }
                                }
                                Some(Pdu::DeliverSm(deliver_sm)) => {
                                    debug!("DeliverSm received.");

                                    for tlv in deliver_sm.tlvs().iter() {
                                        if let TLVTag::ReceiptedMessageId = tlv.tag() {
                                            debug!("Delivery receipt received.");

                                            break 'outer;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                let unbind_command = Command::new(CommandStatus::EsmeRok, 3, Pdu::Unbind);

                framed_write.send(&unbind_command).await.unwrap();

                while let Some(Ok(command)) = framed_read.next().await {
                    if let CommandId::UnbindResp = command.command_id() {
                        debug!("UnbindResp received.");

                        if let CommandStatus::EsmeRok = command.command_status {
                            debug!("Successful unbind.");
                            break;
                        }
                    }
                }
            }
        }

        Ok(StepOutput::Continue)
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| Cow::from(s)).collect()
    }
}

impl Transport for SmppPlugin {
    fn name(&self) -> Cow<'static, str> {
        "smpp".into()
    }

    fn send_step(&self) -> Cow<'static, str> {
        "smpp.send".into()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SmsContent {
    pub body: String,
    pub source_address: String,
}

impl TryFrom<RenderedTemplate> for SmsContent {
    type Error = ();

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0)).map_err(|_| ())
    }
}
