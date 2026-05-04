use async_trait::async_trait;

use crate::{
    config::EventsConfig,
    cooldown::CooldownConfig,
    errors::MrRollerError,
    game::player::PlayerId,
    response::Response,
    store::{EventStore, InventoryStore, ItemUseHistoryStore, LeaderboardStore, PlayerStore},
};

mod admin;
mod common;
mod events;
mod inventory;
mod leaderboard;
mod shop;
mod start;
mod use_item;

pub use admin::{
    AdminAdjustCoinsCommand, AdminGiveItemCommand, AdminHelpCommand, AdminItemKind,
    AdminSetAdminCommand,
};
pub use events::{
    ClaimEventCommand, ListActiveEventsCommand, MaybeSpawnRandomItemEventCommand,
    SpawnRandomItemEventCommand, TrashEventCommand,
};
pub use inventory::InventoryCommand;
pub use leaderboard::LeaderboardCommand;
pub use shop::{BuyItemCommand, ShopCommand, ShopItemKind};
pub use start::StartCommand;
pub use use_item::UseItemCommand;

/// Context provides command handlers with access to all persistent stores.
pub struct Context<'a> {
    pub players: &'a dyn PlayerStore,
    pub inventory: &'a dyn InventoryStore,
    pub leaderboard: &'a dyn LeaderboardStore,
    pub events: &'a dyn EventStore,
    pub item_use_history: &'a dyn ItemUseHistoryStore,
    pub cooldown: &'a CooldownConfig,
    pub bootstrap_admin_ids: &'a [PlayerId],
    pub event_config: &'a EventsConfig,
}

/// Every game action implements Command. The `Output` is converted into a
/// `Response` that frontends can render.
#[async_trait]
pub trait Command: Send {
    type Output: Into<Response>;
    async fn execute(self, ctx: &Context<'_>) -> Result<Self::Output, MrRollerError>;
}

#[cfg(test)]
mod tests;
