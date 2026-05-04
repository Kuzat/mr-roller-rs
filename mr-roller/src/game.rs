pub mod event;
pub mod inventory;
pub mod item;
pub mod player;

use std::sync::Arc;

use crate::{
    command::Command,
    config::EventsConfig,
    cooldown::CooldownConfig,
    game::player::PlayerId,
    response::{Response, ResponseKind},
    store::{
        EventStore, InMemoryEventStore, InMemoryItemUseHistoryStore, InventoryStore,
        ItemUseHistoryStore, LeaderboardStore, PlayerStore,
    },
};

/// Top-level game dispatcher. Frontends (CLI, Discord) only interact with
/// `Game::execute(command)` — they don't touch stores or game logic directly.
pub struct Game {
    players: Arc<dyn PlayerStore>,
    inventory: Arc<dyn InventoryStore>,
    leaderboard: Arc<dyn LeaderboardStore>,
    events: Arc<dyn EventStore>,
    item_use_history: Arc<dyn ItemUseHistoryStore>,
    cooldown: CooldownConfig,
    bootstrap_admin_ids: Vec<PlayerId>,
    event_config: EventsConfig,
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
            events: Arc::new(InMemoryEventStore::new()),
            item_use_history: Arc::new(InMemoryItemUseHistoryStore::new()),
            cooldown,
            bootstrap_admin_ids: Vec::new(),
            event_config: EventsConfig::default(),
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

    pub fn with_event_store(
        players: Arc<dyn PlayerStore>,
        inventory: Arc<dyn InventoryStore>,
        leaderboard: Arc<dyn LeaderboardStore>,
        events: Arc<dyn EventStore>,
        bootstrap_admin_ids: Vec<PlayerId>,
        event_config: EventsConfig,
    ) -> Self {
        Game {
            players,
            inventory,
            leaderboard,
            events,
            item_use_history: Arc::new(InMemoryItemUseHistoryStore::new()),
            cooldown: CooldownConfig::default(),
            bootstrap_admin_ids,
            event_config,
        }
    }

    pub fn with_event_store_and_history(
        players: Arc<dyn PlayerStore>,
        inventory: Arc<dyn InventoryStore>,
        leaderboard: Arc<dyn LeaderboardStore>,
        events: Arc<dyn EventStore>,
        item_use_history: Arc<dyn ItemUseHistoryStore>,
        bootstrap_admin_ids: Vec<PlayerId>,
        event_config: EventsConfig,
    ) -> Self {
        Game {
            players,
            inventory,
            leaderboard,
            events,
            item_use_history,
            cooldown: CooldownConfig::default(),
            bootstrap_admin_ids,
            event_config,
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
            events: self.events.as_ref(),
            item_use_history: self.item_use_history.as_ref(),
            cooldown: &self.cooldown,
            bootstrap_admin_ids: &self.bootstrap_admin_ids,
            event_config: &self.event_config,
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
