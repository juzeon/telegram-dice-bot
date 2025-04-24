mod ano;

use crate::ano::test;
use teloxide::Bot;
use teloxide::payloads::{EditMessageText, GetChat};
use teloxide::prelude::*;
use teloxide::types::Me;
use tracing::{debug, instrument};
use tracing_core::{Level, LevelFilter};
use tracing_subscriber::fmt::layer;
use tracing_subscriber::{Layer, filter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
#[instrument]
async fn main() {
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_filter(LevelFilter::DEBUG)
                .with_filter(filter::filter_fn(|x| {
                    x.target().starts_with(env!("CARGO_PKG_NAME"))
                })),
        )
        .init();
    let bot = Bot::new("token");
    teloxide::repl(bot, |bot: Bot, msg: GetChat, me: Me| async move { Ok(()) }).await;
    println!("Hello, world!");
}
