use std::fmt::Display;

use rand::{thread_rng, Rng};

use super::Usable;

pub trait Dice {
    fn roll(&self) -> u32;
}

impl Usable for dyn Dice {
    fn handle(&self) -> super::output::MrRollerOutput {
        super::output::MrRollerOutput::dice_roll(self.roll())
    }
}

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

impl Dice for BasicDice {
    fn roll(&self) -> u32 {
        thread_rng().gen_range(self.min_roll..=self.max_roll)
    }
}

impl Display for BasicDice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.description)
    }
}
