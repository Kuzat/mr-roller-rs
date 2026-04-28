use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{errors::MrRollerError, game::player::PlayerId};

/// A player's score for leaderboard ranking.
#[derive(Debug, Clone, Default)]
pub struct Score {
    pub xp: u64,
    pub coins: u64,
}

/// Trait for leaderboard persistence.
#[async_trait]
pub trait LeaderboardStore: Send + Sync {
    /// Returns the top `limit` players sorted by score (highest first).
    async fn get_scores(&self, limit: usize) -> Result<Vec<(PlayerId, Score)>, MrRollerError>;

    /// Updates or inserts a player's score.
    async fn update_score(
        &self,
        player_id: PlayerId,
        score: Score,
    ) -> Result<(), MrRollerError>;
}

/// In-memory leaderboard store backed by `HashMap` behind a `RwLock`.
#[derive(Clone, Default)]
pub struct InMemoryLeaderboardStore {
    scores: Arc<RwLock<HashMap<PlayerId, Score>>>,
}

impl InMemoryLeaderboardStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl LeaderboardStore for InMemoryLeaderboardStore {
    async fn get_scores(&self, limit: usize) -> Result<Vec<(PlayerId, Score)>, MrRollerError> {
        let scores = self.scores.read().await;
        let mut entries: Vec<_> = scores.iter().map(|(id, s)| (*id, s.clone())).collect();
        // Sort by XP descending, then coins descending
        entries.sort_by(|a, b| b.1.xp.cmp(&a.1.xp).then(b.1.coins.cmp(&a.1.coins)));
        entries.truncate(limit);
        Ok(entries)
    }

    async fn update_score(&self, player_id: PlayerId, score: Score) -> Result<(), MrRollerError> {
        self.scores.write().await.insert(player_id, score);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_update_and_get_scores() {
        let store = InMemoryLeaderboardStore::new();
        store
            .update_score(PlayerId::new(1), Score { xp: 100, coins: 10 })
            .await
            .unwrap();
        store
            .update_score(PlayerId::new(2), Score { xp: 200, coins: 5 })
            .await
            .unwrap();

        let top = store.get_scores(10).await.unwrap();
        assert_eq!(top.len(), 2);
        // Player 2 should be first (higher XP)
        assert_eq!(top[0].0, PlayerId::new(2));
        assert_eq!(top[1].0, PlayerId::new(1));
    }

    #[tokio::test]
    async fn test_get_scores_respects_limit() {
        let store = InMemoryLeaderboardStore::new();
        for i in 0..5 {
            store
                .update_score(
                    PlayerId::new(i),
                    Score {
                        xp: i * 10,
                        coins: 0,
                    },
                )
                .await
                .unwrap();
        }
        let top = store.get_scores(3).await.unwrap();
        assert_eq!(top.len(), 3);
    }

    #[tokio::test]
    async fn test_empty_leaderboard() {
        let store = InMemoryLeaderboardStore::new();
        let top = store.get_scores(10).await.unwrap();
        assert!(top.is_empty());
    }
}
