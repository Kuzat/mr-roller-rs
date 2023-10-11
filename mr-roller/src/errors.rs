use std::fmt::Display;

#[derive(Debug)]
pub enum MrRollerError {
    PlayerNotFound,
    PlayerAlreadyInGame,
}

impl Display for MrRollerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MrRollerError::PlayerNotFound => write!(f, "Player not found"),
            MrRollerError::PlayerAlreadyInGame => write!(f, "Player already in game"),
        }
    }
}
