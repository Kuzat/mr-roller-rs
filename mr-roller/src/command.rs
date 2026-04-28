use async_trait::async_trait;
use chrono::Utc;

use crate::{
    cooldown::CooldownConfig,
    errors::MrRollerError,
    game::{
        inventory::ItemId,
        item::{dice::basic_dice::BasicDice, Item},
        player::PlayerId,
    },
    response::{Response, ResponseKind},
    store::{InventoryStore, LeaderboardStore, PlayerStore},
};

/// Context provides command handlers with access to all persistent stores.
pub struct Context<'a> {
    pub players: &'a dyn PlayerStore,
    pub inventory: &'a dyn InventoryStore,
    pub leaderboard: &'a dyn LeaderboardStore,
    pub cooldown: &'a CooldownConfig,
}

/// Every game action implements Command. The `Output` is converted into a
/// `Response` that frontends can render.
#[async_trait]
pub trait Command: Send {
    type Output: Into<Response>;
    async fn execute(self, ctx: &Context<'_>) -> Result<Self::Output, MrRollerError>;
}

// ── Concrete commands ──────────────────────────────────────────────────────

// ── Start ──────────────────────────────────────────────────────────────────

pub struct StartCommand {
    pub player_id: PlayerId,
}

#[async_trait]
impl Command for StartCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        if ctx.players.contains(self.player_id).await? {
            return Ok(Response::error("You are already in the game."));
        }

        let player = crate::game::player::Player::new(self.player_id);
        ctx.players.insert(player).await?;

        // Grant starter dice
        ctx.inventory
            .add_item(self.player_id, Item::BasicDice(BasicDice::starter_dice()))
            .await?;

        Ok(Response::success(
            "You have been added to the game and given the starter dice.",
        ))
    }
}

// ── UseItem ────────────────────────────────────────────────────────────────

pub struct UseItemCommand {
    pub player_id: PlayerId,
    pub item_id: ItemId,
}

#[async_trait]
impl Command for UseItemCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        let mut player = ctx.players.get(self.player_id).await?;
        let item = ctx.inventory.get_item(self.player_id, self.item_id).await?;
        let now = Utc::now();

        if item.consumes_daily_roll() {
            if let Some(last_roll_at) = player.last_roll_at {
                if ctx.cooldown.is_on_cooldown(last_roll_at, now) {
                    return Ok(Response::error(
                        "You have already rolled. Your roll cooldown is still active.",
                    ));
                }
            }
        } else {
            // Reroll-style items do not consume the daily roll. In this game,
            // using one clears the player's roll cooldown so they can roll once more.
            // The token itself is consumed from inventory.
            player.last_roll_at = None;
            ctx.players.update(player).await?;
            ctx.inventory.remove_item(self.player_id, self.item_id).await?;
            return Ok(item.handle());
        }

        let response = item.handle();

        // If it was a dice roll, update player roll state and leaderboard.
        if response.kind == ResponseKind::DiceRoll {
            player.last_roll_at = Some(now);

            if let Some(data) = &response.data {
                if let Some(roll) = data.get("roll").and_then(|v| v.as_u64()) {
                    player.xp += roll;
                }
            }

            ctx.players.update(player.clone()).await?;
            ctx.leaderboard
                .update_score(
                    self.player_id,
                    crate::store::leaderboard::Score {
                        xp: player.xp,
                        coins: player.coins,
                    },
                )
                .await?;
        }

        Ok(response)
    }
}

// ── Inventory ──────────────────────────────────────────────────────────────

pub struct InventoryCommand {
    pub player_id: PlayerId,
}

#[async_trait]
impl Command for InventoryCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        ctx.players.get(self.player_id).await?;

        let items = ctx.inventory.list_items(self.player_id).await?;
        let item_list: Vec<serde_json::Value> = items
            .iter()
            .map(|(id, item)| {
                serde_json::json!({
                    "id": id.to_string(),
                    "name": item.name(),
                    "description": item.description(),
                })
            })
            .collect();

        if item_list.is_empty() {
            Ok(Response::inventory(
                "Your inventory is empty.",
                serde_json::json!([]),
            ))
        } else {
            Ok(Response::inventory(
                "Your inventory:",
                serde_json::json!(item_list),
            ))
        }
    }
}

// ── Leaderboard ────────────────────────────────────────────────────────────

pub struct LeaderboardCommand {
    pub limit: usize,
}

#[async_trait]
impl Command for LeaderboardCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        let scores = ctx.leaderboard.get_scores(self.limit).await?;

        let entries: Vec<serde_json::Value> = scores
            .iter()
            .map(|(player_id, score)| {
                serde_json::json!({
                    "player_id": player_id.0,
                    "xp": score.xp,
                    "coins": score.coins,
                })
            })
            .collect();

        if entries.is_empty() {
            Ok(Response::leaderboard("No scores yet.", serde_json::json!([])))
        } else {
            Ok(Response::leaderboard("Leaderboard:", serde_json::json!(entries)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::item::tokens::reroll_token::RerollToken,
        store::{InMemoryInventoryStore, InMemoryLeaderboardStore, InMemoryPlayerStore},
    };

    fn make_context<'a>(
        players: &'a InMemoryPlayerStore,
        inventory: &'a InMemoryInventoryStore,
        leaderboard: &'a InMemoryLeaderboardStore,
        cooldown: &'a CooldownConfig,
    ) -> Context<'a> {
        Context {
            players,
            inventory,
            leaderboard,
            cooldown,
        }
    }

    // ── StartCommand tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_start_new_player() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let cmd = StartCommand {
            player_id: PlayerId::new(1),
        };
        let resp = cmd.execute(&ctx).await.unwrap();
        assert_eq!(resp.kind, ResponseKind::Success);
        assert!(resp.message.contains("starter dice"));

        // Player exists
        assert!(players.contains(PlayerId::new(1)).await.unwrap());

        // Has one item (starter dice)
        let items = inventory.list_items(PlayerId::new(1)).await.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn test_start_duplicate_player() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        // First start succeeds
        StartCommand {
            player_id: PlayerId::new(1),
        }
        .execute(&ctx)
        .await
        .unwrap();

        // Second start returns error
        let resp = StartCommand {
            player_id: PlayerId::new(1),
        }
        .execute(&ctx)
        .await
        .unwrap();
        assert_eq!(resp.kind, ResponseKind::Error);
    }

    // ── UseItemCommand tests ────────────────────────────────────────────

    #[tokio::test]
    async fn test_use_item_dice_roll() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        // Start a player
        StartCommand {
            player_id: PlayerId::new(1),
        }
        .execute(&ctx)
        .await
        .unwrap();

        // Give them a regular dice
        let item_id = inventory
            .add_item(PlayerId::new(1), Item::BasicDice(BasicDice::regular_dice()))
            .await
            .unwrap();

        let resp = UseItemCommand {
            player_id: PlayerId::new(1),
            item_id,
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::DiceRoll);
        assert!(resp.data.is_some());

        // Leaderboard should have an entry
        let scores = leaderboard.get_scores(10).await.unwrap();
        assert_eq!(scores.len(), 1);
    }

    #[tokio::test]
    async fn test_second_dice_roll_same_day_is_blocked() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        StartCommand {
            player_id: PlayerId::new(1),
        }
        .execute(&ctx)
        .await
        .unwrap();

        let item_id = inventory
            .add_item(PlayerId::new(1), Item::BasicDice(BasicDice::regular_dice()))
            .await
            .unwrap();

        let first = UseItemCommand {
            player_id: PlayerId::new(1),
            item_id,
        }
        .execute(&ctx)
        .await
        .unwrap();
        assert_eq!(first.kind, ResponseKind::DiceRoll);

        let second = UseItemCommand {
            player_id: PlayerId::new(1),
            item_id,
        }
        .execute(&ctx)
        .await
        .unwrap();
        assert_eq!(second.kind, ResponseKind::Error);
        assert!(second.message.contains("cooldown"));
    }

    #[tokio::test]
    async fn test_reroll_token_clears_roll_cooldown() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        StartCommand {
            player_id: PlayerId::new(1),
        }
        .execute(&ctx)
        .await
        .unwrap();

        let dice_id = inventory
            .add_item(PlayerId::new(1), Item::BasicDice(BasicDice::regular_dice()))
            .await
            .unwrap();
        let token_id = inventory
            .add_item(PlayerId::new(1), Item::RerollToken(RerollToken::new()))
            .await
            .unwrap();

        let first = UseItemCommand {
            player_id: PlayerId::new(1),
            item_id: dice_id,
        }
        .execute(&ctx)
        .await
        .unwrap();
        assert_eq!(first.kind, ResponseKind::DiceRoll);

        let token = UseItemCommand {
            player_id: PlayerId::new(1),
            item_id: token_id,
        }
        .execute(&ctx)
        .await
        .unwrap();
        assert_eq!(token.kind, ResponseKind::Success);

        let second_roll = UseItemCommand {
            player_id: PlayerId::new(1),
            item_id: dice_id,
        }
        .execute(&ctx)
        .await
        .unwrap();
        assert_eq!(second_roll.kind, ResponseKind::DiceRoll);
    }

    #[tokio::test]
    async fn test_use_item_player_not_found() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let err = UseItemCommand {
            player_id: PlayerId::new(999),
            item_id: uuid::Uuid::new_v4(),
        }
        .execute(&ctx)
        .await
        .unwrap_err();

        assert!(matches!(err, MrRollerError::PlayerNotFound));
    }

    // ── InventoryCommand tests ──────────────────────────────────────────

    #[tokio::test]
    async fn test_inventory_empty() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        StartCommand {
            player_id: PlayerId::new(1),
        }
        .execute(&ctx)
        .await
        .unwrap();

        let resp = InventoryCommand {
            player_id: PlayerId::new(1),
        }
        .execute(&ctx)
        .await
        .unwrap();
        assert_eq!(resp.kind, ResponseKind::Inventory);
    }

    // ── LeaderboardCommand tests ────────────────────────────────────────

    #[tokio::test]
    async fn test_leaderboard_empty() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let resp = LeaderboardCommand { limit: 10 }
            .execute(&ctx)
            .await
            .unwrap();
        assert_eq!(resp.kind, ResponseKind::Leaderboard);
    }
}
