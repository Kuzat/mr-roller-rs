use chrono::Duration;
use std::fmt::Debug;

/// Uniquely identifies a player in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u64);

impl PlayerId {
    pub fn new(id: u64) -> PlayerId {
        PlayerId(id)
    }
}

/// Player is a plain data struct. Inventory is managed separately via
/// `InventoryStore`, scores via `LeaderboardStore`.
#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub cooldown: Duration,
    pub luck: u64,
    pub coins: u64,
    pub xp: u64,
}

impl Player {
    pub fn new(id: PlayerId) -> Player {
        Player {
            id,
            cooldown: Duration::days(1),
            luck: 0,
            coins: 0,
            xp: 0,
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
    }
}
