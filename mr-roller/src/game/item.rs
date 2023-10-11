use std::{collections::HashMap, fmt::Debug};

use crate::output::{self};

pub mod dice;
pub mod tokens;

#[derive(Debug)]
pub enum Item {
    BasicDice(BasicDice),
    RerollToken(RerollToken),
}


pub trait Usable {
    fn handle(&self) -> output::MrRollerOutput;
}

// impl Usable for Item {
//     fn handle(&self) -> output::MrRollerOutput {
//         match self {
//             Item::BasicDice(dice) => MrRollerOutput::dice_roll(dice.roll()),
//         }
//     }
// }

//     CompletedUseable {
//         item: Usable,
//         output: output::MrRollerOutput,
//     },
//     // Item that only used partially and still require further actions
//     UnCompletedUsable {
//         item: Usable,
//         output: output::MrRollerOutput,
//         // TODO: Some more fields for potential actions to be taken
//     },
// }

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

    pub fn get_item(&self, item_id: &ItemId) -> Option<&Item> {
        match self {
            Inventory::LocalInventory(inventory) => inventory.get(item_id),
        }
    }
}
