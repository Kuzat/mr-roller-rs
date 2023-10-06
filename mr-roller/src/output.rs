pub struct MrRollerOutput {
    pub message: String,
    pub color: String,
    // pub player: Option<Player>,
}
impl MrRollerOutput {
    pub(crate) fn dice_roll(roll: u32) -> MrRollerOutput {
        MrRollerOutput {
            message: format!("You rolled a {}", roll),
            color: String::from("green"),
        }
    }
}
