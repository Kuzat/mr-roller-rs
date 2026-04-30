use anyhow::Context as _;
use mr_roller::config::Settings;
use tracing::warn;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod bot;
mod commands;
mod config;
mod events;
mod render;
mod storage;

pub type Error = anyhow::Error;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Clone)]
pub struct Data {
    pub games: storage::DiscordGameRegistry,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let settings = Settings::load().context("failed to load settings")?;
    let discord_config = config::DiscordRuntimeConfig::from_settings(&settings)?;

    if !settings.discord.enabled {
        warn!("discord.enabled is false; starting bot anyway because this binary was invoked directly");
    }

    let registry = storage::DiscordGameRegistry::connect(
        &discord_config.database_url,
        settings.events.clone(),
    )
    .await
    .context("failed to connect to Postgres Discord game registry")?;

    let data = Data { games: registry };

    bot::run_bot(discord_config, data, settings.events.check_interval_seconds).await?;
    Ok(())
}

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "mr_roller_discord=info,serenity=info,poise=info".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
