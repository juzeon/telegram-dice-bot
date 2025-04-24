mod bot;
mod types;

use crate::bot::DiceBot;
use teloxide::prelude::*;
use tracing::{debug, instrument};
use tracing_subscriber::{Layer, filter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    DiceBot::new().await.launch().await;
}
