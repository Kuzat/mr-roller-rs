use crate::{game::item::GameItem, response::Response};
use rand::{thread_rng, Rng};

#[derive(Debug, Clone)]
pub struct CursedDice {
    pub name: String,
    pub description: String,
}

impl CursedDice {
    pub fn new() -> Self {
        CursedDice {
            name: String::from("Cursed Dice"),
            description: String::from("A hexed dice. Rolls are halved but you might get a bonus."),
        }
    }
}

impl Default for CursedDice {
    fn default() -> Self {
        Self::new()
    }
}

impl GameItem for CursedDice {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn handle(&self) -> Response {
        let raw = thread_rng().gen_range(1..=6);
        // 25% chance of a bonus instead of the curse
        if thread_rng().gen_bool(0.25) {
            let bonus = raw * 2;
            Response::dice_roll(
                format!("✨ The curse lifted! You rolled {} (doubled!)", bonus),
                bonus,
            )
        } else {
            let halved = std::cmp::max(1, raw / 2);
            Response::dice_roll(
                format!("👻 Cursed! You rolled {} (halved from {})", halved, raw),
                halved,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ResponseKind;

    #[test]
    fn test_cursed_dice_returns_dice_roll() {
        let dice = CursedDice::new();
        let resp = dice.handle();
        assert_eq!(resp.kind, ResponseKind::DiceRoll);
    }
}
