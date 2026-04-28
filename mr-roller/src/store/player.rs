use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    errors::MrRollerError,
    game::player::{Player, PlayerId},
};

/// Trait for player persistence — swap between in-memory, SQLite, etc.
#[async_trait]
pub trait PlayerStore: Send + Sync {
    /// Returns the player if they exist.
    async fn get(&self, id: PlayerId) -> Result<Player, MrRollerError>;

    /// Inserts a new player. Errors if they already exist.
    async fn insert(&self, player: Player) -> Result<(), MrRollerError>;

    /// Removes a player. Errors if not found.
    async fn remove(&self, id: PlayerId) -> Result<(), MrRollerError>;

    /// Returns true if the player exists.
    async fn contains(&self, id: PlayerId) -> Result<bool, MrRollerError>;

    /// Returns all players.
    async fn all(&self) -> Result<Vec<Player>, MrRollerError>;
}

/// In-memory player store backed by `HashMap` behind a `RwLock`.
#[derive(Clone, Default)]
pub struct InMemoryPlayerStore {
    players: Arc<RwLock<HashMap<PlayerId, Player>>>,
}

impl InMemoryPlayerStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PlayerStore for InMemoryPlayerStore {
    async fn get(&self, id: PlayerId) -> Result<Player, MrRollerError> {
        self.players
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or(MrRollerError::PlayerNotFound)
    }

    async fn insert(&self, player: Player) -> Result<(), MrRollerError> {
        let mut players = self.players.write().await;
        if players.contains_key(&player.id) {
            return Err(MrRollerError::PlayerAlreadyInGame);
        }
        players.insert(player.id, player);
        Ok(())
    }

    async fn remove(&self, id: PlayerId) -> Result<(), MrRollerError> {
        self.players
            .write()
            .await
            .remove(&id)
            .map(|_| ())
            .ok_or(MrRollerError::PlayerNotFound)
    }

    async fn contains(&self, id: PlayerId) -> Result<bool, MrRollerError> {
        Ok(self.players.read().await.contains_key(&id))
    }

    async fn all(&self) -> Result<Vec<Player>, MrRollerError> {
        Ok(self.players.read().await.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::player::Player;

    fn make_player(id: u64) -> Player {
        Player::new(PlayerId::new(id))
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let store = InMemoryPlayerStore::new();
        let player = make_player(1);
        store.insert(player.clone()).await.unwrap();
        let got = store.get(PlayerId::new(1)).await.unwrap();
        assert_eq!(got.id, PlayerId::new(1));
    }

    #[tokio::test]
    async fn test_duplicate_insert() {
        let store = InMemoryPlayerStore::new();
        store.insert(make_player(1)).await.unwrap();
        let err = store.insert(make_player(1)).await.unwrap_err();
        assert!(matches!(err, MrRollerError::PlayerAlreadyInGame));
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let store = InMemoryPlayerStore::new();
        let err = store.get(PlayerId::new(999)).await.unwrap_err();
        assert!(matches!(err, MrRollerError::PlayerNotFound));
    }

    #[tokio::test]
    async fn test_contains() {
        let store = InMemoryPlayerStore::new();
        assert!(!store.contains(PlayerId::new(1)).await.unwrap());
        store.insert(make_player(1)).await.unwrap();
        assert!(store.contains(PlayerId::new(1)).await.unwrap());
    }

    #[tokio::test]
    async fn test_all() {
        let store = InMemoryPlayerStore::new();
        store.insert(make_player(1)).await.unwrap();
        store.insert(make_player(2)).await.unwrap();
        let all = store.all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_remove() {
        let store = InMemoryPlayerStore::new();
        store.insert(make_player(1)).await.unwrap();
        store.remove(PlayerId::new(1)).await.unwrap();
        assert!(!store.contains(PlayerId::new(1)).await.unwrap());
    }

    #[tokio::test]
    async fn test_remove_not_found() {
        let store = InMemoryPlayerStore::new();
        let err = store.remove(PlayerId::new(999)).await.unwrap_err();
        assert!(matches!(err, MrRollerError::PlayerNotFound));
    }
}
