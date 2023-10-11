use std::collections::HashMap;

use crate::errors::MrRollerError;

use super::player::{Player, PlayerId};

pub enum MrRollerState {
    LocalState(HashMap<PlayerId, Player>),
    // DatabaseState(Database),
}

impl MrRollerState {
    pub fn get_player(&self, player_id: PlayerId) -> Result<&Player, MrRollerError> {
        match self {
            MrRollerState::LocalState(state) => match state.get(&player_id) {
                Some(player) => Ok(player),
                None => Err(MrRollerError::PlayerNotFound),
            },
            // MrRollerState::DatabaseState(state) => {
            //     // Get the player from the database
            // }
        }
    }

    pub fn get_player_mut(&mut self, player_id: PlayerId) -> Result<&mut Player, MrRollerError> {
        match self {
            MrRollerState::LocalState(state) => match state.get_mut(&player_id) {
                Some(player) => Ok(player),
                None => Err(MrRollerError::PlayerNotFound),
            },
            // MrRollerState::DatabaseState(state) => {
            //     // Get the player from the database
            // }
        }
    }

    pub fn add_player(&mut self, player: Player) -> Result<(), MrRollerError> {
        match self {
            MrRollerState::LocalState(state) => {
                state.insert(player.id, player);
                Ok(())
            } // MrRollerState::DatabaseState(state) => {
              //     // Add the player to the database
              // }
        }
    }
}
