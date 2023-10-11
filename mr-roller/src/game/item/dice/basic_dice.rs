use std::fmt::Display;

use rand::{thread_rng, Rng};

use crate::{
    game::item::Usable,
    output::{self, MrRollerOutput},
};

#[derive(Debug)]
pub struct BasicDice {
    pub name: String,
    pub description: String,
    min_roll: u32,
    max_roll: u32,
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
            description: String::from("A dice with only 2 sides. Is this even a dice?"),
            min_roll: 1,
            max_roll: 2,
        }
    }
}

impl Usable for BasicDice {
    fn handle(&self) -> MrRollerOutput {
        let random_roll = thread_rng().gen_range(self.min_roll..=self.max_roll);

        MrRollerOutput::DiceRoll(
            output::Base {
                message: format!("You rolled a {}", random_roll),
                color: String::from("green"),
            },
            output::DiceRoll { roll: random_roll },
        )
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
}
