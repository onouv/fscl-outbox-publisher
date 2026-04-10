use sqlx::postgres::PgListener;
use thiserror::Error;
use tracing::info;

use crate::{config::Config, messenger::Messenger};

pub struct Outbox {
    config: Config,
    messenger: Messenger,
}

impl Outbox {
    pub fn new(config: &Config, messenger: Messenger) -> Result<Self, OutboxError> {
        Ok(Self {
            config: config.clone(),
            messenger,
        })
    }

    pub async fn run(&self) -> Result<(), OutboxError> {
        info!("awaiting outbox notification...");

        let mut listener = PgListener::connect(self.config.database_url.as_str())
            .await
            .map_err(|source| OutboxError::ConnectListener { source })?;
        listener
            .listen(self.config.listen_channel.as_str())
            .await
            .map_err(|source| OutboxError::ListenChannel {
                channel: self.config.listen_channel.clone(),
                source,
            })?;

        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(self.config.fallback_poll_ms));
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("shutting down outbox publisher...");
                    break;
                },
                _ = interval.tick() => {
                    let _ = &self.messenger;
                    info!("tidying outbox table...");
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("failed to create LISTEN connection")]
    ConnectListener {
        #[source]
        source: sqlx::Error,
    },

    #[error("failed to LISTEN on channel {channel}")]
    ListenChannel {
        channel: String,
        #[source]
        source: sqlx::Error,
    },

    #[error(transparent)]
    DbError(#[from] sqlx::Error),
}