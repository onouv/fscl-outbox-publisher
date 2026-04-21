use fscl_messaging::{EventEnvelope, OutboxRecord, ensure_outbox_schema};
use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::PgListener;
use sqlx::postgres::PgRow;
use sqlx::types::chrono;
use thiserror::Error;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::{config::Config, messenger::Messenger};

pub struct Outbox {
    pool: sqlx::PgPool,
    config: Config,
    messenger: Messenger,
    }

impl Outbox {
    pub async fn new(config: &Config, messenger: Messenger) -> Result<Self, OutboxError> {
        let pool = PgPool::connect(&config.database_url)
            .await
            .map_err(OutboxError::DbError)?;

        ensure_outbox_schema(&pool)
            .await
            .map_err(|source| OutboxError::EnsureSchema { source })?;

        Ok(Self {
            pool,
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

        self.catch_up_pending_notifications().await?;

        let mut interval = interval(std::time::Duration::from_millis(
            self.config.fallback_poll_ms,
        ));

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("shutting down outbox publisher...");
                    return Ok(());
                },
                _ = interval.tick() => {
                    match self.drain_outbox().await {
                        Ok(count) => {
                            debug!("drained {} outbox records on fallback interval", count);
                            count
                        },
                        Err(e) => {
                            error!("failed_draining_outbox : {}", e);
                            0
                        }
                    };
                }
                recv = listener.recv() => {
                    match recv {
                        Ok(notification) => {
                            debug!("draining outbox on notification: {} ...", notification.payload());
                            match self.drain_outbox().await {
                                Ok(count) => debug!("drained {} outbox records", count),
                                Err(e) => error!("failed_draining_outbox : {}", e),
                            }
                        },
                        Err(e) => {
                            warn!(error = %e, "failed_receiving_notification");
                        }
                    }
                }
            }
        }
    }

    async fn catch_up_pending_notifications(&self) -> Result<(), OutboxError> {
        debug!("catching up on pending outbox notifications...");

        match self.drain_outbox().await {
            Ok(count) => {
                debug!("caught up on {} pending outbox notifications", count);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn drain_outbox(&self) -> Result<usize, OutboxError> {
        let mut tx = self.pool.begin().await?;

        let rows = self.load_records(&mut tx).await?;
        if rows.is_empty() {
            tx.commit().await?;
            return Ok(0);
        }
        
        let mut num_published = 0;
        for row in rows {
            let mut record = Self::outbox_record_from_row(&row)?;

            match self.messenger.publish(&record).await {
                Ok(()) => {
                    record.mark_published(chrono::Utc::now());
                    num_published += 1;
                }
                Err(e) => {
                    error!("failed_publishing_event {}: {}", record.envelope.id, e);
                    record.mark_failure(e.to_string());
                }
            }

            sqlx::query(
                r#"
                UPDATE outbox
                SET published_at = $1, attempts = $2, last_error = $3
                WHERE id = $4
                "#,
            )
            .bind(record.published_at)
            .bind(record.attempts as i32)
            .bind(&record.last_error)
            .bind(record.envelope.id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        
        Ok(num_published)
    }

    async fn load_records(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<Vec<PgRow>, OutboxError> {
        let rows = sqlx::query(
            r#"
            SELECT id, occurred_at, event_type, aggregate_type, aggregate_id, view_id, payload, published_at, attempts, last_error
            FROM outbox
            WHERE published_at IS NULL
            ORDER BY occurred_at
            FOR UPDATE SKIP LOCKED
            LIMIT $1
            "#,
        )
        .bind(self.config.batch_size as i32)
        .fetch_all(&mut **tx)
        .await?;
        Ok(rows)
    }
    
    #[allow(dead_code)]
    pub(crate) fn outbox_record_from_row(row: &PgRow) -> Result<OutboxRecord, OutboxError> {
        let aggregate_type: String = row.try_get("aggregate_type")?;

        Ok(OutboxRecord {
            envelope: EventEnvelope {
                id: row.try_get("id")?,
                occurred_at: row.try_get("occurred_at")?,
                event_type: row.try_get("event_type")?,
                aggregate_type: aggregate_type.parse().map_err(|_| {
                    OutboxError::InvalidAggregateType {
                        value: aggregate_type,
                    }
                })?,
                aggregate_id: row.try_get("aggregate_id")?,
                view_id: row.try_get("view_id")?,
                payload: row.try_get("payload")?,
            },
            published_at: row.try_get("published_at")?,
            attempts: normalize_attempts_from_db(row.try_get("attempts")?)?,
            last_error: row.try_get("last_error")?,
        })
    }
}

#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("failed to ensure outbox schema")]
    EnsureSchema {
        #[source]
        source: sqlx::Error,
    },

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

    #[error("invalid aggregate_type value '{value}' in outbox row")]
    InvalidAggregateType { value: String },

    #[error(transparent)]
    DbError(#[from] sqlx::Error),
}

// postgres stores attempts as signed INT, but we want to work with u32 at domain level. 
// This function converts and validates the value from the database.
#[cfg_attr(not(test), allow(dead_code))]
fn normalize_attempts_from_db(value: i32) -> Result<u32, OutboxError> {
    u32::try_from(value).map_err(|_| OutboxError::NegativeAttempts { value })
}

#[cfg(test)]
mod tests {
    use super::normalize_attempts_from_db;

    #[test]
    fn converts_non_negative_attempts() {
        assert_eq!(normalize_attempts_from_db(0).unwrap(), 0);
        assert_eq!(normalize_attempts_from_db(3).unwrap(), 3);
    }

    #[test]
    fn rejects_negative_attempts() {
        assert!(matches!(
            normalize_attempts_from_db(-1),
            Err(super::OutboxError::NegativeAttempts { value: -1 })
        ));
    }
}
