use fscl_messaging::{EventEnvelope, OutboxRecord};
use sqlx::postgres::PgListener;
use sqlx::postgres::PgRow;
use sqlx::Row;
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

    #[allow(dead_code)]
    pub(crate) fn outbox_record_from_row(row: &PgRow) -> Result<OutboxRecord, OutboxError> {
        Ok(OutboxRecord {
            envelope: EventEnvelope {
                id: row.try_get("id")?,
                occurred_at: row.try_get("occurred_at")?,
                event_type: row.try_get("event_type")?,
                aggregate_type: row.try_get("aggregate_type")?,
                aggregate_id: row.try_get("aggregate_id")?,
                view_id: row.try_get("view_id")?,
                payload: row.try_get("payload")?,
            },
            published_at: row.try_get("published_at")?,
            attempts: attempts_from_db(row.try_get("attempts")?)?,
            last_error: row.try_get("last_error")?,
        })
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn attempts_from_db(value: i32) -> Result<u32, OutboxError> {
    u32::try_from(value).map_err(|_| OutboxError::NegativeAttempts { value })
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

    #[error("outbox attempts must be non-negative, got {value}")]
    NegativeAttempts { value: i32 },

    #[error(transparent)]
    DbError(#[from] sqlx::Error),
}

#[cfg(test)]
mod tests {
    use super::attempts_from_db;

    #[test]
    fn converts_non_negative_attempts() {
        assert_eq!(attempts_from_db(0).unwrap(), 0);
        assert_eq!(attempts_from_db(3).unwrap(), 3);
    }

    #[test]
    fn rejects_negative_attempts() {
        assert!(matches!(
            attempts_from_db(-1),
            Err(super::OutboxError::NegativeAttempts { value: -1 })
        ));
    }
}