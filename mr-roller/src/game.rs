pub mod inventory;
pub mod item;
pub mod player;

use std::sync::Arc;

use crate::{
    command::Command,
    cooldown::CooldownConfig,
    game::player::PlayerId,
    response::{Response, ResponseKind},
    store::{InventoryStore, LeaderboardStore, PlayerStore},
};

/// Top-level game dispatcher. Frontends (CLI, Discord) only interact with
/// `Game::execute(command)` — they don't touch stores or game logic directly.
pub struct Game {
    players: Arc<dyn PlayerStore>,
    inventory: Arc<dyn InventoryStore>,
    leaderboard: Arc<dyn LeaderboardStore>,
    cooldown: CooldownConfig,
    bootstrap_admin_ids: Vec<PlayerId>,
}

impl Game {
    /// Create a new game wired to the provided stores with the default cooldown
    /// config: 24 hours, reset at midnight UTC.
    pub fn new(
        players: Arc<dyn PlayerStore>,
        inventory: Arc<dyn InventoryStore>,
        leaderboard: Arc<dyn LeaderboardStore>,
    ) -> Self {
        Self::with_cooldown(players, inventory, leaderboard, CooldownConfig::default())
    }

    /// Create a new game with an explicit cooldown config.
    pub fn with_cooldown(
        players: Arc<dyn PlayerStore>,
        inventory: Arc<dyn InventoryStore>,
        leaderboard: Arc<dyn LeaderboardStore>,
        cooldown: CooldownConfig,
    ) -> Self {
        Game {
            players,
            inventory,
            leaderboard,
            cooldown,
            bootstrap_admin_ids: Vec::new(),
        }
    }

    /// Create a new game with configured bootstrap admin player IDs.
    pub fn with_bootstrap_admin_ids(
        players: Arc<dyn PlayerStore>,
        inventory: Arc<dyn InventoryStore>,
        leaderboard: Arc<dyn LeaderboardStore>,
        bootstrap_admin_ids: Vec<PlayerId>,
    ) -> Self {
        let mut game = Self::new(players, inventory, leaderboard);
        game.bootstrap_admin_ids = bootstrap_admin_ids;
        game
    }

    /// Execute a command and return a `Response`. Errors from stores or
    /// validation are converted to `Response::Error` so frontends always
    /// receive a renderable result.
    pub async fn execute<C: Command>(&self, cmd: C) -> Response {
        let ctx = crate::command::Context {
            players: self.players.as_ref(),
            inventory: self.inventory.as_ref(),
            leaderboard: self.leaderboard.as_ref(),
            cooldown: &self.cooldown,
            bootstrap_admin_ids: &self.bootstrap_admin_ids,
        };

        match cmd.execute(&ctx).await {
            Ok(output) => output.into(),
            Err(e) => Response {
                kind: ResponseKind::Error,
                message: e.to_string(),
                data: None,
            },
        }
    }
}
