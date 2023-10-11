use std::fmt::Display;

use rand::{thread_rng, Rng};

use crate::output::{self, MrRollerOutput};

use super::Usable;

#[derive(Debug)]
pub struct BasicDice {
    name: String,
    description: String,
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
