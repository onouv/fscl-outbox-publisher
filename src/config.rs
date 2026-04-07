use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub(crate) struct Config {
    database_url: String,
    nats_url: String,
    listen_channel: String,
    subject_prefix: String,
    batch_size: i64,
    fallback_poll_ms: u64,
}

impl Config {
    pub(crate) fn build_database_url_from_env() -> Result<String> {
        let db_type = "postgres".to_string(); 
        let db_host = require_env("DB_HOST")?;
        let db_port = require_env("DB_PORT")?;
        let db_user = require_env("DB_USER")?;
        let db_password = require_env("DB_PASSWORD")?;
        let db_name = require_env("DB_NAME")?;

        Ok(format!(
            "{}://{}:{}@{}:{}/{}",
            db_type, db_user, db_password, db_host, db_port, db_name
        ))
    }

    pub(crate) fn from_env() -> Result<Self> {
        let database_url = Self::build_database_url_from_env()?;

        let nats_url = require_env("NATS_URL")?;
        let listen_channel = require_env("OUTBOX_NOTIFY_CHANNEL")?;
        let subject_prefix = require_env("OUTBOX_SUBJECT_PREFIX")?;

        let batch_size = require_env("OUTBOX_BATCH_SIZE")?
            .parse::<i64>()
            .context("invalid OUTBOX_BATCH_SIZE, expected integer")?;

        let fallback_poll_ms = require_env("OUTBOX_FALLBACK_POLL_MS")?
            .parse::<u64>()
            .context("invalid OUTBOX_FALLBACK_POLL_MS, expected integer")?;

        Ok(Self {
            database_url,
            nats_url,
            listen_channel,
            subject_prefix,
            batch_size,
            fallback_poll_ms,
        })
    }
}

fn require_env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("environment variable {} is required", key))
}