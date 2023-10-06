use std::fmt::Debug;

use super::item::Inventory;

#[derive(Debug)]
pub struct Player {
    pub id: PlayerId,
    pub inventory: Inventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(pub u64);

impl PlayerId {
    pub fn random() -> PlayerId {
        PlayerId(rand::random())
    }
}
