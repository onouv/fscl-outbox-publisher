use fscl_outbox_publisher::{Config, Messenger, Outbox};

use dotenv::{dotenv, from_filename};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    from_filename("../.env.shared").ok();
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "outbox_publisher=info,info".into()),
        )
        .init();

    let config = Config::from_env()?;
    let messenger = Messenger::new(&config).await?; 
    let outbox: Outbox = Outbox::new(&config, messenger)?;

    outbox.run().await?;

    Ok(())
}
