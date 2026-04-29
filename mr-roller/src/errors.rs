use std::fmt::Display;

#[derive(Debug)]
pub enum MrRollerError {
    PlayerNotFound,
    PlayerAlreadyInGame,
    ItemNotFound,
    Storage(String),
}

impl Display for MrRollerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MrRollerError::PlayerNotFound => write!(f, "Player not found"),
            MrRollerError::PlayerAlreadyInGame => write!(f, "Player already in game"),
            MrRollerError::ItemNotFound => write!(f, "Item not found"),
            MrRollerError::Storage(message) => write!(f, "Storage error: {}", message),
        }
    }
}

impl From<sqlx::Error> for MrRollerError {
    fn from(value: sqlx::Error) -> Self {
        MrRollerError::Storage(value.to_string())
    }
}

impl From<serde_json::Error> for MrRollerError {
    fn from(value: serde_json::Error) -> Self {
        MrRollerError::Storage(value.to_string())
    }
}
