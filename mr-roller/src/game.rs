use crate::{
    errors::MrRollerError,
    output::{self, MrRollerOutput},
};

use self::{
    inventory::Inventory,
    item::{dice::basic_dice::BasicDice, Item},
    player::{Player, PlayerId},
    state::MrRollerState,
};

pub mod inventory;
pub mod item;
pub mod player;
pub mod state;

pub struct MrRollerGame {
    // Players map to their respective game state
    state: MrRollerState,
}

impl MrRollerGame {
    pub fn new(state: MrRollerState) -> MrRollerGame {
        MrRollerGame { state }
    }

    fn new_user_inventory(&self) -> Inventory {
        match self.state {
            MrRollerState::LocalState(_) => Inventory::local_inventory(),
        }
    }

    // MVP features for first test.
    // YES /Start adds a player to the game and gives them a starter dice / could give some bonus also
    // YES /Roll rolls the active dice? Do we still want a active dice? Or should we just roll the most
    //      recent dice? Or /Roll <dice_id>? Where it autocompletes the dice_id showing the most recent
    //      first?
    // YES /Use <item_id> Use a item from the inventory, this can also be a dice, and any other items
    //      that have the useable trait. Should also autocomplete show all useable items in the invetory
    //      sorted by most recent first
    // NO  /Inventory shows the inventory of the player
    // YES /Leaderboard Shows the leaderboard of the game
    // NO  /Shop Shows the shop of the game
    // NO  Random events? /Event? /Event <event_id>
    // NO  /Help shows the help menu this should not be part of the game create but rather the
    //     implemntation create
    pub fn start(&mut self, player_id: PlayerId) -> Result<MrRollerOutput, MrRollerError> {
        // Need to check if the player is already in the game
        match self.state.get_player(player_id) {
            Ok(_) => Err(MrRollerError::PlayerAlreadyInGame),
            Err(_) => {
                let player = Player::new(player_id, self.new_user_inventory());
                self.state.add_player(player)?;

                // Fetch the player again from the state
                let player = self.state.get_player_mut(player_id)?;

                // Give the player the starter dice
                player
                    .inventory
                    .add_item(Item::BasicDice(BasicDice::starter_dice()));

                // Return the output
                Ok(MrRollerOutput::Basic(output::Base {
                    message: "You have been added to the game and given the starter dice."
                        .to_string(),
                    color: "green".to_string(),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_start() {
        let mut game = MrRollerGame::new(MrRollerState::LocalState(HashMap::new()));
        let player_id = PlayerId::new(1);
        let output = game.start(player_id).unwrap();
        match output {
            MrRollerOutput::Basic(output::Base { message, color }) => {
                assert_eq!(
                    message,
                    "You have been added to the game and given the starter dice."
                );
                assert_eq!(color, "green");
            }
            _ => panic!("Output is not a basic output"),
        }
        let player = game.state.get_player(player_id).unwrap();
        let items = player.inventory.list_items();
        assert_eq!(items.len(), 1);
        match items.get(0).unwrap() {
            Item::BasicDice(dice) => assert_eq!(dice.name, "Starter Dice"),
            _ => panic!("Item is not a dice"),
        }
    }
}
