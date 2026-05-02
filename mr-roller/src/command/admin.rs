use async_trait::async_trait;

use crate::{
    command::{
        common::{is_bootstrap_admin, leaderboard_score},
        Command, Context,
    },
    errors::MrRollerError,
    game::{
        item::{
            dice::{basic_dice::BasicDice, cursed_dice::CursedDice, lucky_dice::LuckyDice},
            tokens::reroll_token::RerollToken,
            Item,
        },
        player::PlayerId,
    },
    response::{Response, ResponseKind},
};

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

pub(crate) async fn require_admin(
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
            "Admin commands:\n  /admin give <player-id> <item>\n  /admin coins <player-id> <amount>\n  /admin event spawn-random-item\n  /admin set-admin <player-id> <true|false>\nItems: {}",
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
