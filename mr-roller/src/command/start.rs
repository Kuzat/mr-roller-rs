use async_trait::async_trait;

use crate::{
    command::{common::is_bootstrap_admin, Command, Context},
    errors::MrRollerError,
    game::{
        item::{dice::basic_dice::BasicDice, Item},
        player::PlayerId,
    },
    response::Response,
};

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
            let mut player = ctx.players.get(self.player_id).await?;
            let promoted_to_admin = is_bootstrap_admin && !player.is_admin;
            if promoted_to_admin {
                player.is_admin = true;
            }

            if !player.has_started {
                player.has_started = true;
                ctx.players.update(player).await?;
                ctx.inventory
                    .add_item(self.player_id, Item::BasicDice(BasicDice::starter_dice()))
                    .await?;
                return Ok(Response::success(start_tutorial_message()));
            }

            if promoted_to_admin {
                ctx.players.update(player).await?;
                return Ok(Response::success(
                    "You are already in the game and have been promoted to admin.",
                ));
            }
            return Ok(Response::error("You are already in the game."));
        }

        let mut player = crate::game::player::Player::new(self.player_id);
        player.is_admin = is_bootstrap_admin;
        player.has_started = true;
        ctx.players.insert(player).await?;

        // Grant starter dice
        ctx.inventory
            .add_item(self.player_id, Item::BasicDice(BasicDice::starter_dice()))
            .await?;

        Ok(Response::success(start_tutorial_message()))
    }
}

fn start_tutorial_message() -> &'static str {
    "Welcome to Mr Roller! You have been given a starter dice.\n\nUse `/inventory` to see your items, then use `/use` and autocomplete/select your Starter Dice to roll it."
}
