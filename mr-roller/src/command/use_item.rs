use async_trait::async_trait;
use chrono::Utc;

use crate::{
    command::{Command, Context},
    errors::MrRollerError,
    game::{inventory::ItemId, player::PlayerId},
    response::{Response, ResponseKind},
    store::leaderboard::Score,
};

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
        let mut show_tutorial_complete = false;
        let mut roll_amount = None;
        if response.kind == ResponseKind::DiceRoll {
            player.last_roll_at = Some(now);

            if let Some(data) = &response.data {
                if let Some(roll) = data.get("roll").and_then(|v| v.as_u64()) {
                    player.xp += roll;
                    player.coins += roll;
                    roll_amount = Some(roll);
                }
            }

            show_tutorial_complete = player.has_started && !player.tutorial_completed;
            if show_tutorial_complete {
                player.tutorial_completed = true;
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

        if show_tutorial_complete && response.kind == ResponseKind::DiceRoll {
            let mut response = response;
            if let Some(roll) = roll_amount {
                response.message = format!(
                    "{}\n\nYou earned {roll} XP and {roll} gold. XP raises your leaderboard score, and gold can be spent in `/shop` on better dice and useful items. Try `/shop` to see what you can buy, or `/leaderboard` to see the top players.",
                    response.message
                );
            }
            return Ok(response);
        }

        Ok(response)
    }
}
