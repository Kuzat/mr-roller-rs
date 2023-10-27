use chrono::Duration;
use std::fmt::Debug;

use crate::game::Inventory;

#[derive(Debug)]
pub struct Player {
    pub id: PlayerId,
    _cooldown: Duration,
    _luck: u64,
    _coins: u64,
    _xp: u64,
    pub inventory: Inventory,
}

impl Player {
    pub fn new(id: PlayerId, inventory: Inventory) -> Player {
        Player {
            id,
            inventory,
            _cooldown: Duration::days(1),
            _luck: 0,
            _coins: 0,
            _xp: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u64);

impl PlayerId {
    pub fn new(id: u64) -> PlayerId {
        PlayerId(id)
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
        let inventory = Inventory::local_inventory();
        let player = Player::new(id, inventory);
        assert_eq!(player.id, PlayerId(1));
    }
}
