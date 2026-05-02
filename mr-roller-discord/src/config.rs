use anyhow::{anyhow, Context, Result};
use mr_roller::config::Settings;

#[derive(Debug, Clone)]
pub struct DiscordRuntimeConfig {
    pub token: String,
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

        let database_url = settings
            .database
            .url
            .clone()
            .filter(|url| !url.trim().is_empty() && url.starts_with("postgres"))
            .context("the public Discord server requires a PostgreSQL database.url")?;

        Ok(Self {
            token,
            database_url,
        })
    }
}
