use crate::response::Response;

pub mod dice;
pub mod tokens;

#[derive(Debug, Clone)]
pub enum Item {
    BasicDice(dice::basic_dice::BasicDice),
    RerollToken(tokens::reroll_token::RerollToken),
}

impl Item {
    pub fn name(&self) -> &str {
        match self {
            Item::BasicDice(d) => &d.name,
            Item::RerollToken(t) => &t.name,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Item::BasicDice(d) => &d.description,
            Item::RerollToken(t) => &t.description,
        }
    }

    pub fn handle(&self) -> Response {
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
