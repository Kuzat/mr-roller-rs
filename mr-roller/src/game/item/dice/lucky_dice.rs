use crate::{game::item::GameItem, response::Response};
use rand::{thread_rng, Rng};

#[derive(Debug, Clone)]
pub struct LuckyDice {
    pub name: String,
    pub description: String,
}

impl LuckyDice {
    pub fn new() -> Self {
        LuckyDice {
            name: String::from("Lucky Dice"),
            description: String::from("A 20-sided dice. High risk, high reward!"),
        }
    }
}

impl Default for LuckyDice {
    fn default() -> Self {
        Self::new()
    }
}

impl GameItem for LuckyDice {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn handle(&self) -> Response {
        let roll = thread_rng().gen_range(1..=20);
        if roll == 20 {
            Response::dice_roll("🎉 NAT 20! Maximum luck!", roll)
        } else if roll == 1 {
            Response::dice_roll("💀 Critical fail... better luck next time.", roll)
        } else {
            Response::dice_roll(format!("You rolled a {} on the Lucky Dice!", roll), roll)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ResponseKind;

    #[test]
    fn test_lucky_dice_returns_dice_roll() {
        let dice = LuckyDice::new();
        let resp = dice.handle();
        assert_eq!(resp.kind, ResponseKind::DiceRoll);
        let roll = resp.data.unwrap()["roll"].as_u64().unwrap();
        assert!((1..=20).contains(&roll));
    }
}
