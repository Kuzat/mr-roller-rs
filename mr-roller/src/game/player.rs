use chrono::{DateTime, Utc};
use std::fmt::Debug;

/// Uniquely identifies a player in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PlayerId(pub u64);

impl PlayerId {
    pub fn new(id: u64) -> PlayerId {
        PlayerId(id)
    }
}

/// Player is a plain data struct. Inventory is managed separately via
/// `InventoryStore`, scores via `LeaderboardStore`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Player {
    pub id: PlayerId,
    /// When the player last rolled a dice. `None` if they haven't rolled yet.
    pub last_roll_at: Option<DateTime<Utc>>,
    pub luck: u64,
    pub coins: u64,
    pub xp: u64,
    /// Whether this player has run `/start` and received onboarding rewards.
    pub has_started: bool,
    /// Whether this player has completed the initial `/start` tutorial flow.
    pub tutorial_completed: bool,
    /// Whether this player may execute admin-only commands.
    pub is_admin: bool,
}

impl Player {
    pub fn new(id: PlayerId) -> Player {
        Player {
            id,
            last_roll_at: None,
            luck: 0,
            coins: 0,
            xp: 0,
            has_started: false,
            tutorial_completed: false,
            is_admin: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_id() {
        let id = PlayerId::new(1);
        assert_eq!(id, PlayerId(1));
    }

    #[test]
    fn test_player() {
        let id = PlayerId::new(1);
        let player = Player::new(id);
        assert_eq!(player.id, PlayerId(1));
        assert_eq!(player.coins, 0);
        assert_eq!(player.xp, 0);
        assert!(player.last_roll_at.is_none());
        assert!(!player.has_started);
        assert!(!player.tutorial_completed);
        assert!(!player.is_admin);
    }
}
