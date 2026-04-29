use std::fmt::Display;

use crate::{game::item::GameItem, response::Response};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RerollToken {
    pub name: String,
    pub description: String,
    pub amount: u32,
}

impl GameItem for RerollToken {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn handle(&self) -> Response {
        Response::success("Reroll token used — your roll cooldown has been reset.")
    }

    fn consumes_daily_roll(&self) -> bool {
        false
    }
}

impl RerollToken {
    pub fn new() -> RerollToken {
        RerollToken {
            name: String::from("Reroll Token"),
            description: String::from("A token that allows you to reroll a dice"),
            amount: 1,
        }
    }
}

impl Default for RerollToken {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for RerollToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reroll_token() {
        let token = RerollToken::new();
        assert_eq!(token.amount, 1);
    }

    #[test]
    fn test_reroll_token_display() {
        let token = RerollToken::new();
        assert_eq!(
            format!("{}", token),
            "Reroll Token: A token that allows you to reroll a dice"
        );
    }

    #[test]
    fn test_handle() {
        let token = RerollToken::new();
        let resp = token.handle();
        assert_eq!(resp.kind, crate::response::ResponseKind::Success);
    }
}
