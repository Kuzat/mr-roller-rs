use std::{collections::HashMap, fmt::Debug};

use crate::errors::MrRollerError;

use super::item::Item;

pub type ItemId = uuid::Uuid;

pub enum Inventory {
    LocalInventory(HashMap<ItemId, Item>),
    // DatabaseInventory(Database),
}

impl Debug for Inventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Inventory::LocalInventory(inventory) => {
                write!(f, "LocalInventory({:?})", inventory.values())
            }
        }
    }
}

impl Inventory {
    pub fn local_inventory() -> Inventory {
        Inventory::LocalInventory(HashMap::new())
    }

    pub fn add_item(&mut self, item: Item) -> ItemId {
        match self {
            Inventory::LocalInventory(inventory) => {
                let id = uuid::Uuid::new_v4();
                inventory.insert(id, item);
                id
            }
        }
    }

    pub fn get_item(&self, item_id: &ItemId) -> Result<&Item, MrRollerError> {
        match self {
            Inventory::LocalInventory(inventory) => inventory.get(item_id).ok_or(MrRollerError::ItemNotFound),
        }
    }

    pub fn list_items(&self) -> Vec<&Item> {
        match self {
            Inventory::LocalInventory(inventory) => inventory.values().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::game::item::dice::basic_dice::BasicDice;

    use super::*;

    #[test]
    fn test_add_and_get_item() {
        let mut inventory = Inventory::local_inventory();
        let item = Item::BasicDice(BasicDice::regular_dice());
        let item_id = inventory.add_item(item);
        match inventory.get_item(&item_id).unwrap() {
            Item::BasicDice(dice) => assert_eq!(dice.name, "Regular Dice"),
            _ => panic!("Item is not a dice"),
        }
    }
}
