use std::fmt::Display;

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
