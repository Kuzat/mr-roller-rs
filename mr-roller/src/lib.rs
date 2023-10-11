use std::collections::HashMap;

use game::MrRollerGame;

pub mod game;
pub mod errors;
pub mod output;

pub fn init() -> MrRollerGame {
    MrRollerGame::new(game::state::MrRollerState::HashState(HashMap::new()))
}

