use anyhow::{Context, Error};
use async_nats::jetstream::Context as JetStreamContext;
use fscl_messaging::OutboxRecord;
use serde::Serialize;

use crate::config::Config;

pub struct Messenger {
    subject_prefix: String,
    stream: JetStreamContext,
}

impl Messenger {
    pub async fn new(config: &Config) -> Result<Self, Error> {
        let prefix = config.subject_prefix.clone();
        let mut opts = async_nats::ConnectOptions::new();
        if let (Some(user), Some(password)) = (&config.nats_user, &config.nats_password) {
            opts = opts.user_and_password(user.clone(), password.clone());
        }
        let client = opts
            .connect(&config.nats_url)
            .await
            .with_context(|| format!("failed to connect to NATS at {}", config.nats_url))?;
        let stream = async_nats::jetstream::new(client);

        Ok(Self {
            subject_prefix: prefix,
            stream,
        })
    }

    pub async fn publish(&self, event: &OutboxRecord) -> Result<(), MessengerError> {
        let subject = format!("{}.{}", self.subject_prefix, event.envelope.aggregate_type);

        let payload = serde_json::to_vec(&event)?;

        log::debug!("publishing message to subject '{}'", subject);
        match self.stream.publish(subject, payload.into()).await {
            Ok(_) => Ok(()),
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
