use std::fmt::Display;

use crate::{game::item::Usable, output};

#[derive(Debug)]
pub struct RerollToken {
    pub name: String,
    pub description: String,
    pub amount: u32,
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

impl Usable for RerollToken {
    fn handle(&self) -> output::MrRollerOutput {
        output::MrRollerOutput::Basic(output::Base {
            message: "You have used the item.".to_string(),
            color: "green".to_string(),
        })
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
}
