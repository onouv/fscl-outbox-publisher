use anyhow::{Context, Error};
use fscl_messaging::{EventEnvelope, OutboxRecord};
use serde::Serialize;
use async_nats::jetstream::Context as JetStreamContext;


use crate::config::Config;

pub struct Messenger {
    config: Config,
    stream: JetStreamContext 
}

impl Messenger {
    pub async fn new(config: &Config) -> Result<Self, Error> {

        let config = config.clone();
        let client = async_nats::connect(&config.nats_url)
            .await
            .with_context(|| format!("failed to connect to NATS at {}", config.nats_url))?;
        let stream = async_nats::jetstream::new(client);
        
        Ok(Self {
            config,
            stream,
        })
    }

    pub fn publish(&self, event: &OutboxRecord) -> Result<(), MessengerError> {
        let _ = (&self.config, event);
        Ok(())
    }
}

#[derive(Debug, Serialize, thiserror::Error)]
pub enum MessengerError {}