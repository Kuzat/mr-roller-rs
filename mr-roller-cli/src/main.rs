use std::collections::HashMap;

use mr_roller::game::{
    item::{dice::BasicDice, Inventory, Item, Usable},
    player::{Player, PlayerId},
    state::MrRollerState,
    MrRollerGame,
};

fn main() {
    println!("Hello, world!");

    let state = MrRollerState::HashState(HashMap::new());
    let mut game = MrRollerGame::new(state);

    game.add_player(Player {
        id: PlayerId(0),
        inventory: Inventory::local_inventory(),
    })
    .unwrap();

    let player = game.get_player_mut(PlayerId(0)).unwrap();

    println!("{:?}", player);

    let item_id = player.inventory.add_item(Item::regular_dice());

    println!("{:?}", player);

    // Roll
    match player.inventory.get_item(&item_id) {
        None => println!("Item not found"),
        Some(item) => println!("{}", item.handle().message),
    }
}
