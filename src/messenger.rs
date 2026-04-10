use super::config::Config;
use fscl_core::DomainEvent;
use anyhow::Error;
use serde::Serialize;

pub(crate) struct Messenger {
    config: Config
}

impl Messenger {
    pub fn new(config: &Config) -> Result<Self, Error>{
        Ok(Self {
            config: config.clone()
        })
    }


    pub(crate) fn publish( event: DomainEvent) -> Result<(), MessengerError> {
        Ok(())
        
    }
}


#[derive(Debug, Serialize, thiserror::Error)]
pub(crate) enum MessengerError {

}