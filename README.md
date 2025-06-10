# Mr Roller (Rust)

This repository contains a Rust implementation of the **Mr Roller** dice game.  It
is organized as a Cargo workspace with the following crates:

- **mr-roller** – library crate containing all core game logic.
- **mr-roller-cli** – command line interface (currently under development).
- **mr-roller-rs** – workspace root with a simple example binary.

## Building

```bash
# build everything
cargo build --workspace

# build the library only
cargo build -p mr-roller

# build the CLI (may fail until the crate is updated)
cargo build -p mr-roller-cli
```

Run the tests across the workspace with:

```bash
cargo test --workspace
```

## Library usage

The `mr-roller` crate exposes a `MrRollerGame` type which manages game state.  A
new game with local, in-memory state can be created using `mr_roller::init()` or
`MrRollerGame::new`.

```rust
use mr_roller::{init, game::player::PlayerId};

fn main() {
    let mut game = init();
    let output = game.start(PlayerId::new(1)).unwrap();
    println!("{}", match output {
        mr_roller::output::MrRollerOutput::Basic(msg) => msg.message,
        _ => String::new(),
    });
}
```

Items stored in a player's inventory can be activated with
`game.handle(player_id, item_id)`, where `item_id` is obtained when adding the
item to the player's inventory.  See `mr-roller/src/game` for the currently
implemented item types.

## CLI

The `mr-roller-cli` crate aims to provide a lightweight command line
interface.  Compilation currently fails due to outdated imports and will be
addressed in future work.
