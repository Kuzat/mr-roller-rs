use std::fmt::Display;

use rand::{thread_rng, Rng};

use crate::{game::item::GameItem, response::Response};

#[derive(Debug, Clone)]
pub struct BasicDice {
    pub name: String,
    pub description: String,
    pub min_roll: u32,
    pub max_roll: u32,
}

impl GameItem for BasicDice {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn handle(&self) -> Response {
        let random_roll = thread_rng().gen_range(self.min_roll..=self.max_roll);
        Response::dice_roll(format!("You rolled a {}!", random_roll), random_roll)
    }
}

impl BasicDice {
    pub fn regular_dice() -> BasicDice {
        BasicDice {
            name: String::from("Regular Dice"),
            description: String::from("A regular dice with 6 sides"),
            min_roll: 1,
            max_roll: 6,
        }
    }

    pub fn starter_dice() -> BasicDice {
        BasicDice {
            name: String::from("Starter Dice"),
            description: String::from("A dice with only 2 sides. dice or coin?"),
            min_roll: 1,
            max_roll: 2,
        }
    }
}

impl Display for BasicDice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_dice() {
        let dice = BasicDice::regular_dice();
        assert_eq!(dice.min_roll, 1);
        assert_eq!(dice.max_roll, 6);
    }

    #[test]
    fn test_display_basic_dice() {
        let dice = BasicDice::regular_dice();
        assert_eq!(
            format!("{}", dice),
            "Regular Dice: A regular dice with 6 sides"
        );
    }

    #[test]
    fn test_handle_returns_dice_roll() {
        let dice = BasicDice::starter_dice();
        let resp = dice.handle();
        assert_eq!(resp.kind, crate::response::ResponseKind::DiceRoll);
        assert!(resp.data.is_some());
        let roll = resp.data.unwrap()["roll"].as_u64().unwrap();
        assert!(roll == 1 || roll == 2);
    }
}
