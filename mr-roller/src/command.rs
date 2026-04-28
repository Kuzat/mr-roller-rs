use async_trait::async_trait;

use crate::{
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
        // Verify player exists
        ctx.players.get(self.player_id).await?;

        let item = ctx.inventory.get_item(self.player_id, self.item_id).await?;
        let response = item.handle();

        // If it was a dice roll, update the leaderboard
        if response.kind == ResponseKind::DiceRoll {
            if let Some(data) = &response.data {
                if let Some(roll) = data.get("roll").and_then(|v| v.as_u64()) {
                    let mut player = ctx.players.get(self.player_id).await?;
                    player.xp += roll;
                    ctx.players
                        .insert(player.clone())
                        .await
                        .ok(); // ignore AlreadyInGame on update — we know they exist

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
            }
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
    use crate::store::{
        InMemoryInventoryStore, InMemoryLeaderboardStore, InMemoryPlayerStore,
    };

    fn make_context<'a>(
        players: &'a InMemoryPlayerStore,
        inventory: &'a InMemoryInventoryStore,
        leaderboard: &'a InMemoryLeaderboardStore,
    ) -> Context<'a> {
        Context {
            players,
            inventory,
            leaderboard,
        }
    }

    // ── StartCommand tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_start_new_player() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let ctx = make_context(&players, &inventory, &leaderboard);

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
        let ctx = make_context(&players, &inventory, &leaderboard);

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
        let ctx = make_context(&players, &inventory, &leaderboard);

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
    async fn test_use_item_player_not_found() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let ctx = make_context(&players, &inventory, &leaderboard);

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
        let ctx = make_context(&players, &inventory, &leaderboard);

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
        let ctx = make_context(&players, &inventory, &leaderboard);

        let resp = LeaderboardCommand { limit: 10 }
            .execute(&ctx)
            .await
            .unwrap();
        assert_eq!(resp.kind, ResponseKind::Leaderboard);
    }
}
