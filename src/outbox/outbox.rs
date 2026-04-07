use crate::{config::Config, messenger::messenger::Messenger};
use tracing::info;
use thiserror::prelude;

pub(crate) struct Outbox {
    config: Config,
    messenger: Messenger,
}

impl Outbox {
    pub(crate) fn new(config: &Config, messenger: Messenger) -> Result<Self, OutboxError> {
        Ok(Outbox { config: config.clone(), messenger })
    }

    pub(crate) fn observe_and_forward() -> Result<(), OutboxError> {
        info!("entering observation loop...");

        loop {
            
        }

        Ok(())
    }

}

#[derive(Debug, Error)]
pub(crate) enum OutboxError {

    #[error("Cannot connect to database")]
    DbConnectionFailure,

    #[error(transparent)]
    DbError(#[from] sqlx::Error)
}