use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    errors::MrRollerError,
    game::{
        inventory::ItemId,
        item::Item,
        player::PlayerId,
    },
};

/// Trait for inventory persistence.
#[async_trait]
pub trait InventoryStore: Send + Sync {
    /// Get a specific item from a player's inventory.
    async fn get_item(&self, player_id: PlayerId, item_id: ItemId) -> Result<Item, MrRollerError>;

    /// Add an item to a player's inventory, returning the assigned `ItemId`.
    async fn add_item(&self, player_id: PlayerId, item: Item) -> Result<ItemId, MrRollerError>;

    /// Remove a specific item from a player's inventory.
    async fn remove_item(
        &self,
        player_id: PlayerId,
        item_id: ItemId,
    ) -> Result<(), MrRollerError>;

    /// List all items (with their IDs) in a player's inventory.
    async fn list_items(&self, player_id: PlayerId) -> Result<Vec<(ItemId, Item)>, MrRollerError>;
}

/// In-memory inventory store backed by nested `HashMap` behind a `RwLock`.
#[derive(Clone, Default)]
pub struct InMemoryInventoryStore {
    inventories: Arc<RwLock<HashMap<PlayerId, HashMap<ItemId, Item>>>>,
}

impl InMemoryInventoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl InventoryStore for InMemoryInventoryStore {
    async fn get_item(&self, player_id: PlayerId, item_id: ItemId) -> Result<Item, MrRollerError> {
        self.inventories
            .read()
            .await
            .get(&player_id)
            .and_then(|inv| inv.get(&item_id))
            .cloned()
            .ok_or(MrRollerError::ItemNotFound)
    }

    async fn add_item(&self, player_id: PlayerId, item: Item) -> Result<ItemId, MrRollerError> {
        let id = Uuid::new_v4();
        self.inventories
            .write()
            .await
            .entry(player_id)
            .or_default()
            .insert(id, item);
        Ok(id)
    }

    async fn remove_item(
        &self,
        player_id: PlayerId,
        item_id: ItemId,
    ) -> Result<(), MrRollerError> {
        self.inventories
            .write()
            .await
            .get_mut(&player_id)
            .and_then(|inv| inv.remove(&item_id))
            .map(|_| ())
            .ok_or(MrRollerError::ItemNotFound)
    }

    async fn list_items(&self, player_id: PlayerId) -> Result<Vec<(ItemId, Item)>, MrRollerError> {
        Ok(self
            .inventories
            .read()
            .await
            .get(&player_id)
            .map(|inv| inv.clone().into_iter().collect())
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::item::{dice::basic_dice::BasicDice, Item};

    #[tokio::test]
    async fn test_add_and_get_item() {
        let store = InMemoryInventoryStore::new();
        let pid = PlayerId::new(1);
        let item = Item::BasicDice(BasicDice::regular_dice());
        let item_id = store.add_item(pid, item).await.unwrap();
        let got = store.get_item(pid, item_id).await.unwrap();
        assert!(matches!(got, Item::BasicDice(_)));
    }

    #[tokio::test]
    async fn test_get_item_not_found() {
        let store = InMemoryInventoryStore::new();
        let err = store
            .get_item(PlayerId::new(1), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, MrRollerError::ItemNotFound));
    }

    #[tokio::test]
    async fn test_list_items() {
        let store = InMemoryInventoryStore::new();
        let pid = PlayerId::new(1);
        store
            .add_item(pid, Item::BasicDice(BasicDice::regular_dice()))
            .await
            .unwrap();
        store
            .add_item(pid, Item::BasicDice(BasicDice::starter_dice()))
            .await
            .unwrap();
        let items = store.list_items(pid).await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_item() {
        let store = InMemoryInventoryStore::new();
        let pid = PlayerId::new(1);
        let item_id = store
            .add_item(pid, Item::BasicDice(BasicDice::regular_dice()))
            .await
            .unwrap();
        store.remove_item(pid, item_id).await.unwrap();
        let err = store.get_item(pid, item_id).await.unwrap_err();
        assert!(matches!(err, MrRollerError::ItemNotFound));
    }

    #[tokio::test]
    async fn test_remove_not_found() {
        let store = InMemoryInventoryStore::new();
        let err = store
            .remove_item(PlayerId::new(1), Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, MrRollerError::ItemNotFound));
    }
}
