use crate::{config::Config, messenger::Messenger};
use tracing::info;
use thiserror::Error;

pub(crate) struct Outbox {
    config: Config,
    messenger: Messenger,
}

impl Outbox {
    pub(crate) fn new(config: &Config, messenger: Messenger) -> Result<Self, OutboxError> {
        Ok(Outbox { config: config.clone(), messenger })
    }

    pub(crate) async fn observe_and_forward(&self) -> Result<(), OutboxError> {
        info!("observing outbox table...");

        let mut interval = tokio::time::interval(std::time::Duration::from_millis(self.config.fallback_poll_ms));
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("shutting down outbox publisher...");
                    break;
                },
                _ = interval.tick() => {
                    info!("polling outbox table...");
                }            
            }
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