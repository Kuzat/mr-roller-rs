pub mod inventory;
pub mod item;
pub mod player;

use std::sync::Arc;

use crate::{
    command::Command,
    response::{Response, ResponseKind},
    store::{InventoryStore, LeaderboardStore, PlayerStore},
};

/// Top-level game dispatcher. Frontends (CLI, Discord) only interact with
/// `Game::execute(command)` — they don't touch stores or game logic directly.
pub struct Game {
    players: Arc<dyn PlayerStore>,
    inventory: Arc<dyn InventoryStore>,
    leaderboard: Arc<dyn LeaderboardStore>,
}

impl Game {
    /// Create a new game wired to the provided stores. Use the in-memory
    /// implementations for testing/CLI, or swap in database-backed ones later.
    pub fn new(
        players: Arc<dyn PlayerStore>,
        inventory: Arc<dyn InventoryStore>,
        leaderboard: Arc<dyn LeaderboardStore>,
    ) -> Self {
        Game {
            players,
            inventory,
            leaderboard,
        }
    }

    /// Execute a command and return a `Response`. Errors from stores or
    /// validation are converted to `Response::Error` so frontends always
    /// receive a renderable result.
    pub async fn execute<C: Command>(&self, cmd: C) -> Response {
        let ctx = crate::command::Context {
            players: self.players.as_ref(),
            inventory: self.inventory.as_ref(),
            leaderboard: self.leaderboard.as_ref(),
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
