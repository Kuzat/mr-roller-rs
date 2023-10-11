pub enum MrRollerOutput {
    Basic(Base),
    DiceRoll(Base, DiceRoll),
}

pub struct Base {
    pub message: String,
    pub color: String,
    // pub player: Option<Player>,
}
pub struct DiceRoll {
    pub roll: u32,
}
