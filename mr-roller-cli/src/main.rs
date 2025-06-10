use std::collections::HashMap;

use mr_roller::game::{
    inventory::Inventory,
    item::{dice::basic_dice::BasicDice, Item},
    player::{Player, PlayerId},
    state::MrRollerState,
    MrRollerGame,
};

fn main() {
    println!("Hello, world!");

    // Build initial game state with one player and a regular dice.
    let mut state = MrRollerState::LocalState(HashMap::new());
    let mut player = Player::new(PlayerId::new(0), Inventory::local_inventory());
    let item_id = player
        .inventory
        .add_item(Item::BasicDice(BasicDice::regular_dice()));
    state.add_player(player).unwrap();

    let mut game = MrRollerGame::new(state);

    // Use the dice through the game API.
    match game.handle(PlayerId::new(0), item_id).unwrap() {
        mr_roller::output::MrRollerOutput::Basic(msg) => println!("{}", msg.message),
        mr_roller::output::MrRollerOutput::DiceRoll(base, roll) => {
            println!("{}", base.message);
            println!("Rolled {}", roll.roll);
        }
    }
}
