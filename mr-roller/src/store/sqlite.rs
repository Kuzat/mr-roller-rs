use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use uuid::Uuid;

use crate::{
    errors::MrRollerError,
    game::{
        inventory::ItemId,
        item::Item,
        player::{Player, PlayerId},
    },
    store::{inventory::InventoryStore, leaderboard::LeaderboardStore, player::PlayerStore},
};

use super::leaderboard::Score;

/// SQLite-backed store implementing all current store traits.
///
/// Items are stored as JSON using the serializable `Item` enum. This keeps the
/// macro-powered item system easy to extend while still supporting persistence.
#[derive(Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    /// Connect to a SQLite database URL and run schema migration.
    ///
    /// Examples:
    /// - `sqlite:mr-roller.db`
    /// - `sqlite::memory:`
    pub async fn connect(database_url: &str) -> Result<Self, MrRollerError> {
        let pool = SqlitePoolOptions::new()
            // Keep this at 1 so `sqlite::memory:` works reliably too. We can
            // raise it later for file-backed DBs if/when we need more concurrency.
            .max_connections(1)
            .connect(database_url)
            .await?;
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    /// Create a SQLite store from an existing pool and run schema migration.
    pub async fn from_pool(pool: SqlitePool) -> Result<Self, MrRollerError> {
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    /// Convenience helper for tests.
    pub async fn in_memory() -> Result<Self, MrRollerError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        Self::from_pool(pool).await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Applies the current schema. Safe to call multiple times.
    pub async fn migrate(&self) -> Result<(), MrRollerError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS players (
                id INTEGER PRIMARY KEY NOT NULL,
                last_roll_at TEXT NULL,
                luck INTEGER NOT NULL DEFAULT 0,
                coins INTEGER NOT NULL DEFAULT 0,
                xp INTEGER NOT NULL DEFAULT 0,
                is_admin INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        let is_admin_column_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM pragma_table_info('players')
            WHERE name = 'is_admin'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        let has_is_admin = is_admin_column_count > 0;

        if !has_is_admin {
            sqlx::query("ALTER TABLE players ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0")
                .execute(&self.pool)
                .await?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS inventory_items (
                item_id TEXT PRIMARY KEY NOT NULL,
                player_id INTEGER NOT NULL,
                item_json TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(player_id) REFERENCES players(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_inventory_items_player_id
            ON inventory_items(player_id);
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS leaderboard_scores (
                player_id INTEGER PRIMARY KEY NOT NULL,
                xp INTEGER NOT NULL DEFAULT 0,
                coins INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn player_id_to_i64(id: PlayerId) -> i64 {
    id.0 as i64
}

fn i64_to_player_id(id: i64) -> PlayerId {
    PlayerId::new(id as u64)
}

fn opt_datetime_to_string(dt: Option<DateTime<Utc>>) -> Option<String> {
    dt.map(|d| d.to_rfc3339())
}

fn opt_string_to_datetime(value: Option<String>) -> Result<Option<DateTime<Utc>>, MrRollerError> {
    value
        .map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| MrRollerError::Storage(e.to_string()))
        })
        .transpose()
}

fn row_to_player(row: sqlx::sqlite::SqliteRow) -> Result<Player, MrRollerError> {
    Ok(Player {
        id: i64_to_player_id(row.try_get::<i64, _>("id")?),
        last_roll_at: opt_string_to_datetime(row.try_get::<Option<String>, _>("last_roll_at")?)?,
        luck: row.try_get::<i64, _>("luck")? as u64,
        coins: row.try_get::<i64, _>("coins")? as u64,
        xp: row.try_get::<i64, _>("xp")? as u64,
        is_admin: row.try_get::<i64, _>("is_admin")? != 0,
    })
}

#[async_trait]
impl PlayerStore for SqliteStore {
    async fn get(&self, id: PlayerId) -> Result<Player, MrRollerError> {
        let row = sqlx::query(
            r#"
            SELECT id, last_roll_at, luck, coins, xp, is_admin
            FROM players
            WHERE id = ?
            "#,
        )
        .bind(player_id_to_i64(id))
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
            INSERT INTO players (id, last_roll_at, luck, coins, xp, is_admin)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(player_id_to_i64(player.id))
        .bind(opt_datetime_to_string(player.last_roll_at))
        .bind(player.luck as i64)
        .bind(player.coins as i64)
        .bind(player.xp as i64)
        .bind(if player.is_admin { 1_i64 } else { 0_i64 })
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn remove(&self, id: PlayerId) -> Result<(), MrRollerError> {
        let result = sqlx::query("DELETE FROM players WHERE id = ?")
            .bind(player_id_to_i64(id))
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(MrRollerError::PlayerNotFound);
        }
        Ok(())
    }

    async fn contains(&self, id: PlayerId) -> Result<bool, MrRollerError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM players WHERE id = ?")
            .bind(player_id_to_i64(id))
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    async fn all(&self) -> Result<Vec<Player>, MrRollerError> {
        let rows = sqlx::query(
            r#"
            SELECT id, last_roll_at, luck, coins, xp, is_admin
            FROM players
            ORDER BY id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_player).collect()
    }

    async fn update(&self, player: Player) -> Result<(), MrRollerError> {
        let result = sqlx::query(
            r#"
            UPDATE players
            SET last_roll_at = ?, luck = ?, coins = ?, xp = ?, is_admin = ?
            WHERE id = ?
            "#,
        )
        .bind(opt_datetime_to_string(player.last_roll_at))
        .bind(player.luck as i64)
        .bind(player.coins as i64)
        .bind(player.xp as i64)
        .bind(if player.is_admin { 1_i64 } else { 0_i64 })
        .bind(player_id_to_i64(player.id))
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(MrRollerError::PlayerNotFound);
        }
        Ok(())
    }
}

#[async_trait]
impl InventoryStore for SqliteStore {
    async fn get_item(&self, player_id: PlayerId, item_id: ItemId) -> Result<Item, MrRollerError> {
        let item_json: String = sqlx::query_scalar(
            r#"
            SELECT item_json
            FROM inventory_items
            WHERE player_id = ? AND item_id = ?
            "#,
        )
        .bind(player_id_to_i64(player_id))
        .bind(item_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(MrRollerError::ItemNotFound)?;

        Ok(serde_json::from_str(&item_json)?)
    }

    async fn add_item(&self, player_id: PlayerId, item: Item) -> Result<ItemId, MrRollerError> {
        let id = Uuid::new_v4();
        let item_json = serde_json::to_string(&item)?;

        sqlx::query(
            r#"
            INSERT INTO inventory_items (item_id, player_id, item_json)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(player_id_to_i64(player_id))
        .bind(item_json)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn remove_item(&self, player_id: PlayerId, item_id: ItemId) -> Result<(), MrRollerError> {
        let result = sqlx::query(
            r#"
            DELETE FROM inventory_items
            WHERE player_id = ? AND item_id = ?
            "#,
        )
        .bind(player_id_to_i64(player_id))
        .bind(item_id.to_string())
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
            WHERE player_id = ?
            ORDER BY created_at ASC, item_id ASC
            "#,
        )
        .bind(player_id_to_i64(player_id))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let id: String = row.try_get("item_id")?;
                let item_json: String = row.try_get("item_json")?;
                let item_id =
                    Uuid::parse_str(&id).map_err(|e| MrRollerError::Storage(e.to_string()))?;
                let item = serde_json::from_str(&item_json)?;
                Ok((item_id, item))
            })
            .collect()
    }
}

#[async_trait]
impl LeaderboardStore for SqliteStore {
    async fn get_scores(&self, limit: usize) -> Result<Vec<(PlayerId, Score)>, MrRollerError> {
        let rows = sqlx::query(
            r#"
            SELECT player_id, xp, coins
            FROM leaderboard_scores
            ORDER BY xp DESC, coins DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok((
                    i64_to_player_id(row.try_get::<i64, _>("player_id")?),
                    Score {
                        xp: row.try_get::<i64, _>("xp")? as u64,
                        coins: row.try_get::<i64, _>("coins")? as u64,
                    },
                ))
            })
            .collect()
    }

    async fn update_score(&self, player_id: PlayerId, score: Score) -> Result<(), MrRollerError> {
        sqlx::query(
            r#"
            INSERT INTO leaderboard_scores (player_id, xp, coins)
            VALUES (?, ?, ?)
            ON CONFLICT(player_id) DO UPDATE SET
                xp = excluded.xp,
                coins = excluded.coins
            "#,
        )
        .bind(player_id_to_i64(player_id))
        .bind(score.xp as i64)
        .bind(score.coins as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::item::{dice::basic_dice::BasicDice, tokens::reroll_token::RerollToken};

    async fn store() -> SqliteStore {
        SqliteStore::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_player_insert_get_update_all_remove() {
        let store = store().await;
        let mut player = Player::new(PlayerId::new(1));
        player.xp = 10;
        store.insert(player.clone()).await.unwrap();

        let got = store.get(PlayerId::new(1)).await.unwrap();
        assert_eq!(got.id, PlayerId::new(1));
        assert_eq!(got.xp, 10);

        let duplicate = store
            .insert(Player::new(PlayerId::new(1)))
            .await
            .unwrap_err();
        assert!(matches!(duplicate, MrRollerError::PlayerAlreadyInGame));

        player.xp = 99;
        store.update(player).await.unwrap();
        assert_eq!(store.get(PlayerId::new(1)).await.unwrap().xp, 99);

        let all = store.all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert!(store.contains(PlayerId::new(1)).await.unwrap());

        store.remove(PlayerId::new(1)).await.unwrap();
        assert!(!store.contains(PlayerId::new(1)).await.unwrap());
    }

    #[tokio::test]
    async fn test_inventory_add_get_list_remove() {
        let store = store().await;
        let pid = PlayerId::new(1);
        store.insert(Player::new(pid)).await.unwrap();

        let dice_id = store
            .add_item(pid, Item::BasicDice(BasicDice::regular_dice()))
            .await
            .unwrap();
        let token_id = store
            .add_item(pid, Item::RerollToken(RerollToken::new()))
            .await
            .unwrap();

        assert!(matches!(
            store.get_item(pid, dice_id).await.unwrap(),
            Item::BasicDice(_)
        ));
        assert!(matches!(
            store.get_item(pid, token_id).await.unwrap(),
            Item::RerollToken(_)
        ));

        let items = store.list_items(pid).await.unwrap();
        assert_eq!(items.len(), 2);

        store.remove_item(pid, token_id).await.unwrap();
        let items = store.list_items(pid).await.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn test_leaderboard_update_and_get_scores() {
        let store = store().await;
        store
            .update_score(PlayerId::new(1), Score { xp: 10, coins: 0 })
            .await
            .unwrap();
        store
            .update_score(PlayerId::new(2), Score { xp: 20, coins: 0 })
            .await
            .unwrap();

        let scores = store.get_scores(10).await.unwrap();
        assert_eq!(scores.len(), 2);
        assert_eq!(scores[0].0, PlayerId::new(2));
    }
}
