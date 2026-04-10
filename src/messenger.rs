use anyhow::Error;
use fscl_core::DomainEvent;
use serde::Serialize;

use crate::config::Config;

pub struct Messenger {
    config: Config,
}

impl Messenger {
    pub fn new(config: &Config) -> Result<Self, Error> {
        Ok(Self {
            config: config.clone(),
        })
    }

    pub fn publish(&self, event: DomainEvent) -> Result<(), MessengerError> {
        let _ = (&self.config, event);
        Ok(())
    }
}

#[derive(Debug, Serialize, thiserror::Error)]
pub enum MessengerError {}