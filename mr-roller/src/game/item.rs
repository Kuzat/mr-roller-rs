use crate::output::{self};

pub mod dice;
pub mod tokens;

#[derive(Debug)]
pub enum Item {
    BasicDice(dice::basic_dice::BasicDice),
    RerollToken(tokens::reroll_token::RerollToken),
}

pub trait Usable {
    fn handle(&self) -> output::MrRollerOutput;
}

impl Usable for Item {
    fn handle(&self) -> output::MrRollerOutput {
        match self {
            Item::BasicDice(dice) => dice.handle(),
            Item::RerollToken(token) => token.handle(),
        }
    }
}

//     CompletedUseable {
//         item: Usable,
//         output: output::MrRollerOutput,
//     },
//     // Item that only used partially and still require further actions
//     UnCompletedUsable {
//         item: Usable,
//         output: output::MrRollerOutput,
//         // TODO: Some more fields for potential actions to be taken
//     },
// }
