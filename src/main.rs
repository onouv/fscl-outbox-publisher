mod config;
use config::*;

mod messenger;
use messenger::*;

mod outbox;
use outbox::*;

use dotenv::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "outbox_publisher=info,info".into()),
        )
        .init();

    let config = Config::from_env()?;
    let messenger = Messenger::new(&config)?; 
    let outbox: Outbox = Outbox::new(&config, messenger)?;

    outbox.observe_and_forward().await?;

    Ok(())
}
