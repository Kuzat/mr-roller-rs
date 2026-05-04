use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mr_roller::{
    errors::MrRollerError,
    game::{
        event::{ActiveEvent, EventId, EventStatus},
        inventory::ItemId,
        item::Item,
        player::{Player, PlayerId},
    },
    store::{
        event::EventStore,
        history::{ItemUseHistoryStore, ItemUseRecord},
        inventory::InventoryStore,
        leaderboard::{LeaderboardStore, Score},
        player::PlayerStore,
    },
};
use sqlx::{postgres::PgRow, PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PostgresGameStore {
    pool: PgPool,
    game_id: Uuid,
}

impl PostgresGameStore {
    pub fn new(pool: PgPool, game_id: Uuid) -> Self {
        Self { pool, game_id }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn game_id(&self) -> Uuid {
        self.game_id
    }
}

fn player_id_to_text(id: PlayerId) -> String {
    id.0.to_string()
}

fn text_to_player_id(id: String) -> Result<PlayerId, MrRollerError> {
    id.parse::<u64>()
        .map(PlayerId::new)
        .map_err(|error| MrRollerError::Storage(error.to_string()))
}

fn u64_to_i64(value: u64, field: &str) -> Result<i64, MrRollerError> {
    i64::try_from(value)
        .map_err(|_| MrRollerError::Storage(format!("{field} is too large for PostgreSQL BIGINT")))
}

fn i64_to_u64(value: i64, field: &str) -> Result<u64, MrRollerError> {
    u64::try_from(value).map_err(|_| MrRollerError::Storage(format!("{field} is negative")))
}

fn row_to_player(row: PgRow) -> Result<Player, MrRollerError> {
    Ok(Player {
        id: text_to_player_id(row.try_get::<String, _>("id")?)?,
        last_roll_at: row.try_get::<Option<DateTime<Utc>>, _>("last_roll_at")?,
        luck: i64_to_u64(row.try_get::<i64, _>("luck")?, "luck")?,
        coins: i64_to_u64(row.try_get::<i64, _>("coins")?, "coins")?,
        xp: i64_to_u64(row.try_get::<i64, _>("xp")?, "xp")?,
        has_started: row.try_get("has_started")?,
        tutorial_completed: row.try_get("tutorial_completed")?,
        is_admin: row.try_get("is_admin")?,
    })
}

fn event_status_key(status: &EventStatus) -> &'static str {
    match status {
        EventStatus::Active => "active",
        EventStatus::Claimed { .. } => "claimed",
        EventStatus::Trashed { .. } => "trashed",
        EventStatus::Expired => "expired",
    }
}

#[async_trait]
impl PlayerStore for PostgresGameStore {
    async fn get(&self, id: PlayerId) -> Result<Player, MrRollerError> {
        let row = sqlx::query(
            r#"
            SELECT id, last_roll_at, luck, coins, xp, has_started, tutorial_completed, is_admin
            FROM players
            WHERE game_id = $1 AND id = $2
            "#,
        )
        .bind(self.game_id)
        .bind(player_id_to_text(id))
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MrRollerError::PlayerNotFound)?;

        row_to_player(row)
    }

    async fn insert(&self, player: Player) -> Result<(), MrRollerError> {
        if self.contains(player.id).await? {
            return Err(MrRollerError::PlayerAlreadyInGame);
        }

        sqlx::query(
            r#"
            INSERT INTO players (game_id, id, last_roll_at, luck, coins, xp, has_started, tutorial_completed, is_admin)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(self.game_id)
        .bind(player_id_to_text(player.id))
        .bind(player.last_roll_at)
        .bind(u64_to_i64(player.luck, "luck")?)
        .bind(u64_to_i64(player.coins, "coins")?)
        .bind(u64_to_i64(player.xp, "xp")?)
        .bind(player.has_started)
        .bind(player.tutorial_completed)
        .bind(player.is_admin)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove(&self, id: PlayerId) -> Result<(), MrRollerError> {
        let result = sqlx::query("DELETE FROM players WHERE game_id = $1 AND id = $2")
            .bind(self.game_id)
            .bind(player_id_to_text(id))
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(MrRollerError::PlayerNotFound);
        }
        Ok(())
    }

    async fn contains(&self, id: PlayerId) -> Result<bool, MrRollerError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM players WHERE game_id = $1 AND id = $2")
                .bind(self.game_id)
                .bind(player_id_to_text(id))
                .fetch_one(&self.pool)
                .await?;
        Ok(count > 0)
    }

    async fn all(&self) -> Result<Vec<Player>, MrRollerError> {
        let rows = sqlx::query(
            r#"
            SELECT id, last_roll_at, luck, coins, xp, has_started, tutorial_completed, is_admin
            FROM players
            WHERE game_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(self.game_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_player).collect()
    }

    async fn update(&self, player: Player) -> Result<(), MrRollerError> {
        let result = sqlx::query(
            r#"
            UPDATE players
            SET last_roll_at = $3, luck = $4, coins = $5, xp = $6, has_started = $7, tutorial_completed = $8, is_admin = $9
            WHERE game_id = $1 AND id = $2
            "#,
        )
        .bind(self.game_id)
        .bind(player_id_to_text(player.id))
        .bind(player.last_roll_at)
        .bind(u64_to_i64(player.luck, "luck")?)
        .bind(u64_to_i64(player.coins, "coins")?)
        .bind(u64_to_i64(player.xp, "xp")?)
        .bind(player.has_started)
        .bind(player.tutorial_completed)
        .bind(player.is_admin)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(MrRollerError::PlayerNotFound);
        }
        Ok(())
    }
}

#[async_trait]
impl InventoryStore for PostgresGameStore {
    async fn get_item(&self, player_id: PlayerId, item_id: ItemId) -> Result<Item, MrRollerError> {
        let value: serde_json::Value = sqlx::query_scalar(
            r#"
            SELECT item_json
            FROM inventory_items
            WHERE game_id = $1 AND player_id = $2 AND item_id = $3
            "#,
        )
        .bind(self.game_id)
        .bind(player_id_to_text(player_id))
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MrRollerError::ItemNotFound)?;
        Ok(serde_json::from_value(value)?)
    }

    async fn add_item(&self, player_id: PlayerId, item: Item) -> Result<ItemId, MrRollerError> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO inventory_items (game_id, item_id, player_id, item_json)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(self.game_id)
        .bind(id)
        .bind(player_id_to_text(player_id))
        .bind(serde_json::to_value(item)?)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn remove_item(&self, player_id: PlayerId, item_id: ItemId) -> Result<(), MrRollerError> {
        let result = sqlx::query(
            r#"
            DELETE FROM inventory_items
            WHERE game_id = $1 AND player_id = $2 AND item_id = $3
            "#,
        )
        .bind(self.game_id)
        .bind(player_id_to_text(player_id))
        .bind(item_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(MrRollerError::ItemNotFound);
        }
        Ok(())
    }

    async fn list_items(&self, player_id: PlayerId) -> Result<Vec<(ItemId, Item)>, MrRollerError> {
        let rows = sqlx::query(
            r#"
            SELECT item_id, item_json
            FROM inventory_items
            WHERE game_id = $1 AND player_id = $2
            ORDER BY created_at ASC, item_id ASC
            "#,
        )
        .bind(self.game_id)
        .bind(player_id_to_text(player_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let id: Uuid = row.try_get("item_id")?;
                let value: serde_json::Value = row.try_get("item_json")?;
                Ok((id, serde_json::from_value(value)?))
            })
            .collect()
    }
}

#[async_trait]
impl LeaderboardStore for PostgresGameStore {
    async fn get_scores(&self, limit: usize) -> Result<Vec<(PlayerId, Score)>, MrRollerError> {
        let rows = sqlx::query(
            r#"
            SELECT player_id, xp, coins
            FROM leaderboard_scores
            WHERE game_id = $1
            ORDER BY xp DESC, coins DESC
            LIMIT $2
            "#,
        )
        .bind(self.game_id)
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok((
                    text_to_player_id(row.try_get::<String, _>("player_id")?)?,
                    Score {
                        xp: i64_to_u64(row.try_get::<i64, _>("xp")?, "xp")?,
                        coins: i64_to_u64(row.try_get::<i64, _>("coins")?, "coins")?,
                    },
                ))
            })
            .collect()
    }

    async fn update_score(&self, player_id: PlayerId, score: Score) -> Result<(), MrRollerError> {
        sqlx::query(
            r#"
            INSERT INTO leaderboard_scores (game_id, player_id, xp, coins)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT(game_id, player_id) DO UPDATE SET
                xp = excluded.xp,
                coins = excluded.coins
            "#,
        )
        .bind(self.game_id)
        .bind(player_id_to_text(player_id))
        .bind(u64_to_i64(score.xp, "xp")?)
        .bind(u64_to_i64(score.coins, "coins")?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl ItemUseHistoryStore for PostgresGameStore {
    async fn record_item_use(&self, record: ItemUseRecord) -> Result<(), MrRollerError> {
        sqlx::query(
            r#"
            INSERT INTO item_use_history (game_id, id, player_id, item_id, item_name, item_kind, item_json, response_kind, roll, used_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(self.game_id)
        .bind(record.id)
        .bind(player_id_to_text(record.player_id))
        .bind(record.item_id)
        .bind(record.item_name)
        .bind(record.item_kind)
        .bind(record.item_json)
        .bind(record.response_kind)
        .bind(record.roll.map(|roll| u64_to_i64(roll, "roll")).transpose()?)
        .bind(record.used_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_item_uses(
        &self,
        player_id: PlayerId,
        limit: usize,
    ) -> Result<Vec<ItemUseRecord>, MrRollerError> {
        let rows = sqlx::query(
            r#"
            SELECT id, player_id, item_id, item_name, item_kind, item_json, response_kind, roll, used_at
            FROM item_use_history
            WHERE game_id = $1 AND player_id = $2
            ORDER BY used_at DESC, id DESC
            LIMIT $3
            "#,
        )
        .bind(self.game_id)
        .bind(player_id_to_text(player_id))
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ItemUseRecord {
                    id: row.try_get("id")?,
                    player_id: text_to_player_id(row.try_get::<String, _>("player_id")?)?,
                    item_id: row.try_get("item_id")?,
                    item_name: row.try_get("item_name")?,
                    item_kind: row.try_get("item_kind")?,
                    item_json: row.try_get("item_json")?,
                    response_kind: row.try_get("response_kind")?,
                    roll: row
                        .try_get::<Option<i64>, _>("roll")?
                        .map(|roll| i64_to_u64(roll, "roll"))
                        .transpose()?,
                    used_at: row.try_get("used_at")?,
                })
            })
            .collect()
    }
}

#[async_trait]
impl EventStore for PostgresGameStore {
    async fn insert_event(&self, event: ActiveEvent) -> Result<(), MrRollerError> {
        sqlx::query(
            r#"
            INSERT INTO active_events (game_id, id, kind, event_json, status, created_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(self.game_id)
        .bind(event.id)
        .bind(event.title())
        .bind(serde_json::to_value(&event)?)
        .bind(event_status_key(&event.status))
        .bind(event.created_at)
        .bind(event.expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_event(&self, id: EventId) -> Result<ActiveEvent, MrRollerError> {
        let value: serde_json::Value = sqlx::query_scalar(
            "SELECT event_json FROM active_events WHERE game_id = $1 AND id = $2",
        )
        .bind(self.game_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| MrRollerError::Storage("Event not found".to_string()))?;
        Ok(serde_json::from_value(value)?)
    }

    async fn update_event(&self, event: ActiveEvent) -> Result<(), MrRollerError> {
        let result = sqlx::query(
            r#"
            UPDATE active_events
            SET event_json = $3, status = $4, expires_at = $5
            WHERE game_id = $1 AND id = $2
            "#,
        )
        .bind(self.game_id)
        .bind(event.id)
        .bind(serde_json::to_value(&event)?)
        .bind(event_status_key(&event.status))
        .bind(event.expires_at)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(MrRollerError::Storage("Event not found".to_string()));
        }
        Ok(())
    }

    async fn list_events(&self) -> Result<Vec<ActiveEvent>, MrRollerError> {
        let rows = sqlx::query(
            r#"
            SELECT event_json
            FROM active_events
            WHERE game_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(self.game_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let value: serde_json::Value = row.try_get("event_json")?;
                Ok(serde_json::from_value(value)?)
            })
            .collect()
    }
}
