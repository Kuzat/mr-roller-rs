use std::collections::HashMap;

use game::MrRollerGame;

pub mod errors;
pub mod game;
pub mod output;

pub fn init() -> MrRollerGame {
    MrRollerGame::new(game::state::MrRollerState::LocalState(HashMap::new()))
}
