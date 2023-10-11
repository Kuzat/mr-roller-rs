use crate::{errors::MrRollerError, output::MrRollerOutput};

use self::{
    item::ItemId,
    player::{Player, PlayerId},
    state::MrRollerState,
};

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

    pub fn add_player(&mut self, player: Player) -> Result<(), MrRollerError> {
        // Add the player to the game state (Which most likely is a database or internal map)
        self.state.add_player(player)
    }

    pub fn get_player(&self, player_id: PlayerId) -> Result<&Player, MrRollerError> {
        // Get the player from the game state (Which most likely is a database or internal map)
        self.state.get_player(player_id)
    }

    pub fn get_player_mut(&mut self, player_id: PlayerId) -> Result<&mut Player, MrRollerError> {
        // Get the player from the game state (Which most likely is a database or internal map)
        self.state.get_player_mut(player_id)
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


}
