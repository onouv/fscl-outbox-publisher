
use anyhow::{Context, Error};
use fscl_messaging::OutboxRecord;
use serde::Serialize;
use async_nats::jetstream::Context as JetStreamContext;

use crate::config::Config;

pub struct Messenger {
    subject_prefix: String,
    stream: JetStreamContext
}

impl Messenger {
    pub async fn new(config: &Config) -> Result<Self, Error> {

        let prefix = config.subject_prefix.clone();
        let client = async_nats::connect(&config.nats_url)
            .await
            .with_context(|| format!("failed to connect to NATS at {}", config.nats_url))?;
        let stream = async_nats::jetstream::new(client);
        
        Ok(Self {
            subject_prefix: prefix,
            stream,
        })
    }

    pub async fn publish(&self, event: &OutboxRecord) -> Result<(), MessengerError> {

        let subject = format!(
            "{}.{}.{}",
            self.subject_prefix, event.envelope.aggregate_type, event.envelope.event_type
        );

        let payload = serde_json::to_vec(&event)?;

        match self.stream.publish(subject, payload.into()).await {
            Ok(_) =>  return Ok(()),
            Err(e) => Err(MessengerError::PublishError(e.to_string())),
        }
    }
}

#[derive(Debug, Serialize, thiserror::Error)]
pub enum MessengerError {
    #[error("failed to publish message: {0}")]
    PublishError(String),
}

impl From<serde_json::Error> for MessengerError {
    fn from(err: serde_json::Error) -> Self {
        MessengerError::PublishError(format!("failed to serialize message payload: {}", err))
    }
}

