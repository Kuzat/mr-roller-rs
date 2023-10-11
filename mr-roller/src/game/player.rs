use std::fmt::Debug;

use crate::game::Inventory;

#[derive(Debug)]
pub struct Player {
    pub id: PlayerId,
    pub inventory: Inventory,
}

impl Player {
    pub fn new(id: PlayerId, inventory: Inventory) -> Player {
        Player { id, inventory }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u64);

impl PlayerId {
    pub fn new(id: u64) -> PlayerId {
        PlayerId(id)
    }
}
