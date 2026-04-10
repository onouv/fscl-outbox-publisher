use crate::{config::Config, messenger::Messenger};
use sqlx::postgres::PgListener;
use thiserror::Error;
use tracing::info;

pub(crate) struct Outbox {
    config: Config,
    messenger: Messenger,
}

impl Outbox {
    pub(crate) fn new(config: &Config, messenger: Messenger) -> Result<Self, OutboxError> {
        Ok(Outbox { config: config.clone(), messenger })
    }

    /** Await notifications from the outbox table in database, periodically tidy the outbox table.
      
    TODO: implement graceful shutdown and error handling, and publish events to NATS when notification is received.
    */
    pub(crate) async fn run(&self) -> Result<(), OutboxError> {
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

        let mut interval = tokio::time::interval(std::time::Duration::from_millis(self.config.fallback_poll_ms));
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("shutting down outbox publisher...");
                    break;
                },
                _ = interval.tick() => {
                    info!("tidying outbox table...");
                }            
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub(crate) enum OutboxError {
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