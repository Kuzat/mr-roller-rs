use serde::Serialize;

/// A structured response returned by every game command.
/// Frontends (CLI, Discord) render these without knowing game internals.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub kind: ResponseKind,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Categorises the response so frontends can render appropriately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseKind {
    Success,
    Error,
    DiceRoll,
    Inventory,
    Leaderboard,
    Shop,
    Event,
}

impl Response {
    pub fn success(message: impl Into<String>) -> Self {
        Response {
            kind: ResponseKind::Success,
            message: message.into(),
            data: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Response {
            kind: ResponseKind::Error,
            message: message.into(),
            data: None,
        }
    }

    pub fn dice_roll(message: impl Into<String>, roll: u32) -> Self {
        Response {
            kind: ResponseKind::DiceRoll,
            message: message.into(),
            data: Some(serde_json::json!({ "roll": roll })),
        }
    }

    pub fn inventory(message: impl Into<String>, items: serde_json::Value) -> Self {
        Response {
            kind: ResponseKind::Inventory,
            message: message.into(),
            data: Some(items),
        }
    }

    pub fn leaderboard(message: impl Into<String>, entries: serde_json::Value) -> Self {
        Response {
            kind: ResponseKind::Leaderboard,
            message: message.into(),
            data: Some(entries),
        }
    }

    pub fn shop(message: impl Into<String>, entries: serde_json::Value) -> Self {
        Response {
            kind: ResponseKind::Shop,
            message: message.into(),
            data: Some(entries),
        }
    }

    pub fn event(message: impl Into<String>, data: serde_json::Value) -> Self {
        Response {
            kind: ResponseKind::Event,
            message: message.into(),
            data: Some(data),
        }
    }
}
