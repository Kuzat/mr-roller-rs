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
}
