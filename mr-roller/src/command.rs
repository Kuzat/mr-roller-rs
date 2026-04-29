use async_trait::async_trait;
use chrono::Utc;

use crate::{
    cooldown::CooldownConfig,
    errors::MrRollerError,
    game::{
        inventory::ItemId,
        item::{
            dice::{basic_dice::BasicDice, cursed_dice::CursedDice, lucky_dice::LuckyDice},
            tokens::reroll_token::RerollToken,
            Item,
        },
        player::PlayerId,
    },
    response::{Response, ResponseKind},
    store::{leaderboard::Score, InventoryStore, LeaderboardStore, PlayerStore},
};

/// Context provides command handlers with access to all persistent stores.
pub struct Context<'a> {
    pub players: &'a dyn PlayerStore,
    pub inventory: &'a dyn InventoryStore,
    pub leaderboard: &'a dyn LeaderboardStore,
    pub cooldown: &'a CooldownConfig,
    pub bootstrap_admin_ids: &'a [PlayerId],
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
        let is_bootstrap_admin = is_bootstrap_admin(self.player_id, ctx.bootstrap_admin_ids);

        if ctx.players.contains(self.player_id).await? {
            if is_bootstrap_admin {
                let mut player = ctx.players.get(self.player_id).await?;
                if !player.is_admin {
                    player.is_admin = true;
                    ctx.players.update(player).await?;
                    return Ok(Response::success(
                        "You are already in the game and have been promoted to admin.",
                    ));
                }
            }
            return Ok(Response::error("You are already in the game."));
        }

        let mut player = crate::game::player::Player::new(self.player_id);
        player.is_admin = is_bootstrap_admin;
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
            ctx.inventory
                .remove_item(self.player_id, self.item_id)
                .await?;
            return Ok(item.handle());
        }

        let response = item.handle();

        // If it was a dice roll, update player roll state and leaderboard.
        if response.kind == ResponseKind::DiceRoll {
            player.last_roll_at = Some(now);

            if let Some(data) = &response.data {
                if let Some(roll) = data.get("roll").and_then(|v| v.as_u64()) {
                    player.xp += roll;
                    player.coins += roll;
                }
            }

            ctx.players.update(player.clone()).await?;
            ctx.leaderboard
                .update_score(
                    self.player_id,
                    Score {
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

// ── Shop helpers ───────────────────────────────────────────────────────────

/// Item types players can buy from the shop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopItemKind {
    StarterDice,
    RegularDice,
    LuckyDice,
    CursedDice,
}

pub struct ShopCatalogEntry {
    pub key: &'static str,
    pub item: ShopItemKind,
    pub price: u64,
}

const SHOP_CATALOG: &[ShopCatalogEntry] = &[
    ShopCatalogEntry {
        key: "starter_dice",
        item: ShopItemKind::StarterDice,
        price: 5,
    },
    ShopCatalogEntry {
        key: "regular_dice",
        item: ShopItemKind::RegularDice,
        price: 25,
    },
    ShopCatalogEntry {
        key: "lucky_dice",
        item: ShopItemKind::LuckyDice,
        price: 100,
    },
    ShopCatalogEntry {
        key: "cursed_dice",
        item: ShopItemKind::CursedDice,
        price: 50,
    },
];

impl ShopItemKind {
    pub fn keys() -> Vec<&'static str> {
        SHOP_CATALOG.iter().map(|entry| entry.key).collect()
    }

    pub fn catalog_entry(&self) -> &'static ShopCatalogEntry {
        SHOP_CATALOG
            .iter()
            .find(|entry| entry.item == *self)
            .expect("shop item must exist in catalog")
    }
}

impl std::str::FromStr for ShopItemKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "starter" | "starter_dice" => Ok(Self::StarterDice),
            "regular" | "regular_dice" | "basic" | "basic_dice" => Ok(Self::RegularDice),
            "lucky" | "lucky_dice" => Ok(Self::LuckyDice),
            "cursed" | "cursed_dice" => Ok(Self::CursedDice),
            other => Err(format!(
                "Unknown shop item '{}'. Available items: {}",
                other,
                Self::keys().join(", ")
            )),
        }
    }
}

impl From<ShopItemKind> for Item {
    fn from(kind: ShopItemKind) -> Self {
        match kind {
            ShopItemKind::StarterDice => Item::BasicDice(BasicDice::starter_dice()),
            ShopItemKind::RegularDice => Item::BasicDice(BasicDice::regular_dice()),
            ShopItemKind::LuckyDice => Item::LuckyDice(LuckyDice::new()),
            ShopItemKind::CursedDice => Item::CursedDice(CursedDice::new()),
        }
    }
}

fn leaderboard_score(player: &crate::game::player::Player) -> Score {
    Score {
        xp: player.xp,
        coins: player.coins,
    }
}

// ── Shop ───────────────────────────────────────────────────────────────────

pub struct ShopCommand;

#[async_trait]
impl Command for ShopCommand {
    type Output = Response;

    async fn execute(self, _ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        let entries: Vec<serde_json::Value> = SHOP_CATALOG
            .iter()
            .map(|entry| {
                let item: Item = entry.item.into();
                serde_json::json!({
                    "key": entry.key,
                    "name": item.name(),
                    "description": item.description(),
                    "price": entry.price,
                })
            })
            .collect();

        Ok(Response::shop("Shop:", serde_json::json!(entries)))
    }
}

// ── BuyItem ────────────────────────────────────────────────────────────────

pub struct BuyItemCommand {
    pub player_id: PlayerId,
    pub item: ShopItemKind,
}

#[async_trait]
impl Command for BuyItemCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        let mut player = ctx.players.get(self.player_id).await?;
        let catalog_entry = self.item.catalog_entry();

        if player.coins < catalog_entry.price {
            return Ok(Response::error(format!(
                "You need {} coins to buy {} but only have {}.",
                catalog_entry.price, catalog_entry.key, player.coins
            )));
        }

        let item: Item = self.item.into();
        let item_name = item.name().to_string();
        player.coins -= catalog_entry.price;

        ctx.players.update(player.clone()).await?;
        ctx.inventory.add_item(self.player_id, item).await?;
        ctx.leaderboard
            .update_score(self.player_id, leaderboard_score(&player))
            .await?;

        Ok(Response::success(format!(
            "Bought {} for {} coins. You now have {} coins.",
            item_name, catalog_entry.price, player.coins
        )))
    }
}

// ── Admin helpers ──────────────────────────────────────────────────────────

/// Item types that admin commands can grant to players.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminItemKind {
    StarterDice,
    RegularDice,
    LuckyDice,
    CursedDice,
    RerollToken,
}

impl AdminItemKind {
    pub fn keys() -> &'static [&'static str] {
        &[
            "starter_dice",
            "regular_dice",
            "lucky_dice",
            "cursed_dice",
            "reroll_token",
        ]
    }
}

impl std::str::FromStr for AdminItemKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "starter" | "starter_dice" => Ok(Self::StarterDice),
            "regular" | "regular_dice" | "basic" | "basic_dice" => Ok(Self::RegularDice),
            "lucky" | "lucky_dice" => Ok(Self::LuckyDice),
            "cursed" | "cursed_dice" => Ok(Self::CursedDice),
            "reroll" | "reroll_token" | "token" => Ok(Self::RerollToken),
            other => Err(format!(
                "Unknown item '{}'. Available items: {}",
                other,
                Self::keys().join(", ")
            )),
        }
    }
}

impl From<AdminItemKind> for Item {
    fn from(kind: AdminItemKind) -> Self {
        match kind {
            AdminItemKind::StarterDice => Item::BasicDice(BasicDice::starter_dice()),
            AdminItemKind::RegularDice => Item::BasicDice(BasicDice::regular_dice()),
            AdminItemKind::LuckyDice => Item::LuckyDice(LuckyDice::new()),
            AdminItemKind::CursedDice => Item::CursedDice(CursedDice::new()),
            AdminItemKind::RerollToken => Item::RerollToken(RerollToken::new()),
        }
    }
}

fn is_bootstrap_admin(player_id: PlayerId, bootstrap_admin_ids: &[PlayerId]) -> bool {
    bootstrap_admin_ids.contains(&player_id)
}

async fn require_admin(
    ctx: &Context<'_>,
    player_id: PlayerId,
) -> Result<Option<Response>, MrRollerError> {
    let player = ctx.players.get(player_id).await?;
    if player.is_admin || is_bootstrap_admin(player_id, ctx.bootstrap_admin_ids) {
        Ok(None)
    } else {
        Ok(Some(Response::error(
            "You do not have permission to use admin commands.",
        )))
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
            Ok(Response::leaderboard(
                "No scores yet.",
                serde_json::json!([]),
            ))
        } else {
            Ok(Response::leaderboard(
                "Leaderboard:",
                serde_json::json!(entries),
            ))
        }
    }
}

// ── AdminHelp ──────────────────────────────────────────────────────────────

pub struct AdminHelpCommand {
    pub player_id: PlayerId,
}

#[async_trait]
impl Command for AdminHelpCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        if let Some(response) = require_admin(ctx, self.player_id).await? {
            return Ok(response);
        }

        Ok(Response::success(format!(
            "Admin commands: /admin give <player-id> <item>, /admin coins <player-id> <amount>, /admin set-admin <player-id> <true|false>. Items: {}",
            AdminItemKind::keys().join(", ")
        )))
    }
}

// ── AdminGiveItem ──────────────────────────────────────────────────────────

pub struct AdminGiveItemCommand {
    pub admin_id: PlayerId,
    pub target_player_id: PlayerId,
    pub item: AdminItemKind,
}

#[async_trait]
impl Command for AdminGiveItemCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        if let Some(response) = require_admin(ctx, self.admin_id).await? {
            return Ok(response);
        }

        if !ctx.players.contains(self.target_player_id).await? {
            ctx.players
                .insert(crate::game::player::Player::new(self.target_player_id))
                .await?;
        }

        let item: Item = self.item.into();
        let item_name = item.name().to_string();
        let item_id = ctx.inventory.add_item(self.target_player_id, item).await?;

        Ok(Response {
            kind: ResponseKind::Success,
            message: format!("Gave {} to player {}.", item_name, self.target_player_id.0),
            data: Some(serde_json::json!({
                "player_id": self.target_player_id.0,
                "item_id": item_id.to_string(),
                "item_name": item_name,
            })),
        })
    }
}

// ── AdminAdjustCoins ───────────────────────────────────────────────────────

pub struct AdminAdjustCoinsCommand {
    pub admin_id: PlayerId,
    pub target_player_id: PlayerId,
    pub amount: i64,
}

#[async_trait]
impl Command for AdminAdjustCoinsCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        if let Some(response) = require_admin(ctx, self.admin_id).await? {
            return Ok(response);
        }

        let mut target = ctx.players.get(self.target_player_id).await?;
        if self.amount < 0 {
            let amount_to_remove = self.amount.unsigned_abs();
            if target.coins < amount_to_remove {
                return Ok(Response::error(format!(
                    "Player {} only has {} coins.",
                    self.target_player_id.0, target.coins
                )));
            }
            target.coins -= amount_to_remove;
        } else {
            target.coins += self.amount as u64;
        }

        ctx.players.update(target.clone()).await?;
        ctx.leaderboard
            .update_score(self.target_player_id, leaderboard_score(&target))
            .await?;

        Ok(Response::success(format!(
            "Player {} now has {} coins.",
            self.target_player_id.0, target.coins
        )))
    }
}

// ── AdminSetAdmin ──────────────────────────────────────────────────────────

pub struct AdminSetAdminCommand {
    pub admin_id: PlayerId,
    pub target_player_id: PlayerId,
    pub is_admin: bool,
}

#[async_trait]
impl Command for AdminSetAdminCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        if let Some(response) = require_admin(ctx, self.admin_id).await? {
            return Ok(response);
        }

        let mut target = ctx.players.get(self.target_player_id).await?;
        target.is_admin = self.is_admin;
        ctx.players.update(target).await?;

        Ok(Response::success(format!(
            "Player {} is now {}admin.",
            self.target_player_id.0,
            if self.is_admin { "an " } else { "not an " }
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{InMemoryInventoryStore, InMemoryLeaderboardStore, InMemoryPlayerStore};

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
            bootstrap_admin_ids: &[],
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
    async fn test_start_bootstrap_admin() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let bootstrap_admin_ids = [PlayerId::new(7)];
        let ctx = Context {
            players: &players,
            inventory: &inventory,
            leaderboard: &leaderboard,
            cooldown: &cooldown,
            bootstrap_admin_ids: &bootstrap_admin_ids,
        };

        StartCommand {
            player_id: PlayerId::new(7),
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert!(players.get(PlayerId::new(7)).await.unwrap().is_admin);
    }

    #[tokio::test]
    async fn test_start_promotes_existing_bootstrap_admin() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let bootstrap_admin_ids = [PlayerId::new(7)];
        let ctx = Context {
            players: &players,
            inventory: &inventory,
            leaderboard: &leaderboard,
            cooldown: &cooldown,
            bootstrap_admin_ids: &bootstrap_admin_ids,
        };

        players
            .insert(crate::game::player::Player::new(PlayerId::new(7)))
            .await
            .unwrap();

        let resp = StartCommand {
            player_id: PlayerId::new(7),
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Success);
        assert!(players.get(PlayerId::new(7)).await.unwrap().is_admin);
    }

    #[tokio::test]
    async fn test_bootstrap_admin_can_use_admin_command_even_before_persisted_flag() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let bootstrap_admin_ids = [PlayerId::new(7)];
        let ctx = Context {
            players: &players,
            inventory: &inventory,
            leaderboard: &leaderboard,
            cooldown: &cooldown,
            bootstrap_admin_ids: &bootstrap_admin_ids,
        };

        players
            .insert(crate::game::player::Player::new(PlayerId::new(7)))
            .await
            .unwrap();

        let resp = AdminHelpCommand {
            player_id: PlayerId::new(7),
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Success);
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
        let data = resp.data.as_ref().unwrap();

        let player = players.get(PlayerId::new(1)).await.unwrap();
        let roll = data["roll"].as_u64().unwrap();
        assert_eq!(player.coins, roll);
        assert_eq!(player.xp, roll);

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

    // ── ShopCommand tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_shop_lists_dice_without_reroll_token() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let resp = ShopCommand.execute(&ctx).await.unwrap();
        assert_eq!(resp.kind, ResponseKind::Shop);
        let items = resp.data.unwrap().as_array().cloned().unwrap();
        assert!(items.iter().any(|item| item["key"] == "regular_dice"));
        assert!(items.iter().any(|item| item["key"] == "lucky_dice"));
        assert!(items.iter().any(|item| item["key"] == "cursed_dice"));
        assert!(!items.iter().any(|item| item["key"] == "reroll_token"));
    }

    #[tokio::test]
    async fn test_buy_item_spends_coins_and_adds_item() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let mut player = crate::game::player::Player::new(PlayerId::new(1));
        player.coins = 100;
        players.insert(player).await.unwrap();

        let resp = BuyItemCommand {
            player_id: PlayerId::new(1),
            item: ShopItemKind::LuckyDice,
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Success);
        assert_eq!(players.get(PlayerId::new(1)).await.unwrap().coins, 0);
        let items = inventory.list_items(PlayerId::new(1)).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0].1, Item::LuckyDice(_)));
    }

    #[tokio::test]
    async fn test_buy_item_requires_enough_coins() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        players
            .insert(crate::game::player::Player::new(PlayerId::new(1)))
            .await
            .unwrap();

        let resp = BuyItemCommand {
            player_id: PlayerId::new(1),
            item: ShopItemKind::LuckyDice,
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Error);
        assert!(inventory
            .list_items(PlayerId::new(1))
            .await
            .unwrap()
            .is_empty());
    }

    // ── AdminCommand tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_give_item_requires_admin() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        players
            .insert(crate::game::player::Player::new(PlayerId::new(1)))
            .await
            .unwrap();
        players
            .insert(crate::game::player::Player::new(PlayerId::new(2)))
            .await
            .unwrap();

        let resp = AdminGiveItemCommand {
            admin_id: PlayerId::new(1),
            target_player_id: PlayerId::new(2),
            item: AdminItemKind::RerollToken,
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Error);
        assert!(inventory
            .list_items(PlayerId::new(2))
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn test_admin_give_item_adds_item_to_target() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let mut admin = crate::game::player::Player::new(PlayerId::new(1));
        admin.is_admin = true;
        players.insert(admin).await.unwrap();
        players
            .insert(crate::game::player::Player::new(PlayerId::new(2)))
            .await
            .unwrap();

        let resp = AdminGiveItemCommand {
            admin_id: PlayerId::new(1),
            target_player_id: PlayerId::new(2),
            item: AdminItemKind::LuckyDice,
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Success);
        let items = inventory.list_items(PlayerId::new(2)).await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0].1, Item::LuckyDice(_)));
    }

    #[tokio::test]
    async fn test_admin_give_item_creates_missing_target() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let mut admin = crate::game::player::Player::new(PlayerId::new(1));
        admin.is_admin = true;
        players.insert(admin).await.unwrap();

        let resp = AdminGiveItemCommand {
            admin_id: PlayerId::new(1),
            target_player_id: PlayerId::new(99),
            item: AdminItemKind::RerollToken,
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Success);
        assert!(players.contains(PlayerId::new(99)).await.unwrap());
        assert_eq!(
            inventory.list_items(PlayerId::new(99)).await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn test_admin_adjust_coins_adds_and_removes_coins() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let mut admin = crate::game::player::Player::new(PlayerId::new(1));
        admin.is_admin = true;
        players.insert(admin).await.unwrap();
        players
            .insert(crate::game::player::Player::new(PlayerId::new(2)))
            .await
            .unwrap();

        AdminAdjustCoinsCommand {
            admin_id: PlayerId::new(1),
            target_player_id: PlayerId::new(2),
            amount: 25,
        }
        .execute(&ctx)
        .await
        .unwrap();

        let resp = AdminAdjustCoinsCommand {
            admin_id: PlayerId::new(1),
            target_player_id: PlayerId::new(2),
            amount: -10,
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Success);
        assert_eq!(players.get(PlayerId::new(2)).await.unwrap().coins, 15);
    }

    #[tokio::test]
    async fn test_admin_adjust_coins_cannot_remove_more_than_player_has() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let mut admin = crate::game::player::Player::new(PlayerId::new(1));
        admin.is_admin = true;
        players.insert(admin).await.unwrap();
        players
            .insert(crate::game::player::Player::new(PlayerId::new(2)))
            .await
            .unwrap();

        let resp = AdminAdjustCoinsCommand {
            admin_id: PlayerId::new(1),
            target_player_id: PlayerId::new(2),
            amount: -1,
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Error);
        assert_eq!(players.get(PlayerId::new(2)).await.unwrap().coins, 0);
    }

    #[tokio::test]
    async fn test_admin_set_admin_updates_target() {
        let players = InMemoryPlayerStore::new();
        let inventory = InMemoryInventoryStore::new();
        let leaderboard = InMemoryLeaderboardStore::new();
        let cooldown = CooldownConfig::default();
        let ctx = make_context(&players, &inventory, &leaderboard, &cooldown);

        let mut admin = crate::game::player::Player::new(PlayerId::new(1));
        admin.is_admin = true;
        players.insert(admin).await.unwrap();
        players
            .insert(crate::game::player::Player::new(PlayerId::new(2)))
            .await
            .unwrap();

        let resp = AdminSetAdminCommand {
            admin_id: PlayerId::new(1),
            target_player_id: PlayerId::new(2),
            is_admin: true,
        }
        .execute(&ctx)
        .await
        .unwrap();

        assert_eq!(resp.kind, ResponseKind::Success);
        assert!(players.get(PlayerId::new(2)).await.unwrap().is_admin);
    }
}
