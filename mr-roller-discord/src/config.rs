use anyhow::{anyhow, Context, Result};
use mr_roller::config::Settings;
use serenity::all::{ChannelId, GuildId};

#[derive(Debug, Clone)]
pub struct DiscordRuntimeConfig {
    pub token: String,
    pub guild_id: Option<GuildId>,
    pub home_channel_id: ChannelId,
    pub database_url: String,
}

impl DiscordRuntimeConfig {
    pub fn from_settings(settings: &Settings) -> Result<Self> {
        let token = settings
            .discord
            .token
            .clone()
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| anyhow!("missing discord.token (or MR_ROLLER__DISCORD__TOKEN)"))?;

        let home_channel_id = settings
            .discord
            .home_channel_id
            .filter(|id| *id != 0)
            .map(ChannelId::new)
            .ok_or_else(|| anyhow!("missing discord.home_channel_id"))?;

        let database_url = settings
            .database
            .url
            .clone()
            .filter(|url| !url.trim().is_empty() && url != "sqlite::memory:")
            .context("the Discord server requires a file-backed SQLite database url")?;

        Ok(Self {
            token,
            guild_id: settings
                .discord
                .guild_id
                .filter(|id| *id != 0)
                .map(GuildId::new),
            home_channel_id,
            database_url,
        })
    }
}
