use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use mr_roller::{config::EventsConfig, game::Game};
use serenity::all::{ChannelId, GuildId, UserId};
use sqlx::{migrate::Migrator, postgres::PgPoolOptions, PgPool, Row};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::PostgresGameStore;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone)]
pub struct DiscordGame {
    pub game_id: Uuid,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub created_by_user_id: UserId,
    pub events_enabled: bool,
}

#[derive(Clone)]
pub struct ResolvedDiscordGame {
    pub discord_game: DiscordGame,
    pub game: Arc<Game>,
    pub store: Arc<PostgresGameStore>,
}

#[derive(Clone)]
pub struct DiscordGameRegistry {
    pool: PgPool,
    event_config: EventsConfig,
    cache: Arc<RwLock<HashMap<Uuid, ResolvedDiscordGame>>>,
}

impl DiscordGameRegistry {
    pub async fn connect(database_url: &str, event_config: EventsConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        MIGRATOR.run(&pool).await?;
        Ok(Self::new(pool, event_config))
    }

    pub fn new(pool: PgPool, event_config: EventsConfig) -> Self {
        Self {
            pool,
            event_config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn setup_game(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        created_by: UserId,
    ) -> Result<(DiscordGame, bool)> {
        let mut tx = self.pool.begin().await?;
        let guild_text = guild_id.get().to_string();
        let channel_text = channel_id.get().to_string();
        let user_text = created_by.get().to_string();

        let existing = sqlx::query(
            r#"
            SELECT game_id, guild_id, channel_id, created_by_user_id, events_enabled
            FROM discord_games
            WHERE guild_id = $1 AND channel_id = $2
            "#,
        )
        .bind(&guild_text)
        .bind(&channel_text)
        .fetch_optional(&mut *tx)
        .await?;

        let (row, created) = if let Some(row) = existing {
            (row, false)
        } else {
            let game_id = Uuid::new_v4();
            let row = sqlx::query(
                r#"
                INSERT INTO discord_games (game_id, guild_id, channel_id, created_by_user_id)
                VALUES ($1, $2, $3, $4)
                RETURNING game_id, guild_id, channel_id, created_by_user_id, events_enabled
                "#,
            )
            .bind(game_id)
            .bind(&guild_text)
            .bind(&channel_text)
            .bind(&user_text)
            .fetch_one(&mut *tx)
            .await?;
            (row, true)
        };

        let discord_game = row_to_discord_game(row)?;
        sqlx::query(
            r#"
            INSERT INTO players (game_id, id, is_admin)
            VALUES ($1, $2, true)
            ON CONFLICT (game_id, id) DO UPDATE SET is_admin = true
            "#,
        )
        .bind(discord_game.game_id)
        .bind(&user_text)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        self.cache.write().await.remove(&discord_game.game_id);
        Ok((discord_game, created))
    }

    pub async fn game_for_channel(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Result<Option<ResolvedDiscordGame>> {
        let row = sqlx::query(
            r#"
            SELECT game_id, guild_id, channel_id, created_by_user_id, events_enabled
            FROM discord_games
            WHERE guild_id = $1 AND channel_id = $2
            "#,
        )
        .bind(guild_id.get().to_string())
        .bind(channel_id.get().to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.game_for_row(row_to_discord_game(row)?).await)),
            None => Ok(None),
        }
    }

    pub async fn game_for_id(&self, game_id: Uuid) -> Result<Option<ResolvedDiscordGame>> {
        if let Some(cached) = self.cache.read().await.get(&game_id).cloned() {
            return Ok(Some(cached));
        }
        let row = sqlx::query(
            r#"
            SELECT game_id, guild_id, channel_id, created_by_user_id, events_enabled
            FROM discord_games
            WHERE game_id = $1
            "#,
        )
        .bind(game_id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => Ok(Some(self.game_for_row(row_to_discord_game(row)?).await)),
            None => Ok(None),
        }
    }

    pub async fn list_games_with_events_enabled(&self) -> Result<Vec<DiscordGame>> {
        let rows = sqlx::query(
            r#"
            SELECT game_id, guild_id, channel_id, created_by_user_id, events_enabled
            FROM discord_games
            WHERE events_enabled = true
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_discord_game).collect()
    }

    pub async fn player_count(&self, game_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar("SELECT COUNT(*) FROM players WHERE game_id = $1")
            .bind(game_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    async fn game_for_row(&self, discord_game: DiscordGame) -> ResolvedDiscordGame {
        if let Some(cached) = self.cache.read().await.get(&discord_game.game_id).cloned() {
            return cached;
        }

        let store = Arc::new(PostgresGameStore::new(
            self.pool.clone(),
            discord_game.game_id,
        ));
        let game = Arc::new(Game::with_event_store(
            store.clone(),
            store.clone(),
            store.clone(),
            store.clone(),
            Vec::new(),
            self.event_config.clone(),
        ));
        let resolved = ResolvedDiscordGame {
            discord_game: discord_game.clone(),
            game,
            store,
        };
        self.cache
            .write()
            .await
            .insert(discord_game.game_id, resolved.clone());
        resolved
    }
}

fn row_to_discord_game(row: sqlx::postgres::PgRow) -> Result<DiscordGame> {
    Ok(DiscordGame {
        game_id: row.try_get("game_id")?,
        guild_id: GuildId::new(parse_snowflake(
            row.try_get::<String, _>("guild_id")?,
            "guild_id",
        )?),
        channel_id: ChannelId::new(parse_snowflake(
            row.try_get::<String, _>("channel_id")?,
            "channel_id",
        )?),
        created_by_user_id: UserId::new(parse_snowflake(
            row.try_get::<String, _>("created_by_user_id")?,
            "created_by_user_id",
        )?),
        events_enabled: row.try_get("events_enabled")?,
    })
}

fn parse_snowflake(value: String, field: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .map_err(|error| anyhow!("invalid {field} snowflake {value}: {error}"))
}
