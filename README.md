# Mr Roller (Rust)

This repository contains a Rust implementation of the **Mr Roller** dice game. It
is organized as a Cargo workspace with the following crates:

- **mr-roller** — library crate containing all core game logic.
- **mr-roller-cli** — interactive command-line REPL.
- **mr-roller-rs** — workspace root with a simple example binary.

## Architecture

The library is built around three pluggable layers:

| Layer | Purpose | Swap for... |
|---|---|---|
| **Stores** (`PlayerStore`, `InventoryStore`, `LeaderboardStore`) | Persistence | In-memory → SQLite, PostgreSQL |
| **Commands** (`StartCommand`, `UseItemCommand`, …) | Game actions | Add new commands without touching existing code |
| **Items** (`define_items!` macro) | Dice, tokens, etc. | One struct + one macro line per new item |
| **Cooldowns** (`CooldownConfig`) | Roll limits | Default: 24h cooldown, reset at midnight UTC |

Frontends (CLI, Discord) only interact with `Game::execute(command)` — they never
touch stores or game logic directly.

## Building

```bash
# build everything
cargo build --workspace

# build the library only
cargo build -p mr-roller

# build the CLI
cargo build -p mr-roller-cli
```

Run the tests across the workspace with:

```bash
cargo test --workspace
```

## CLI

Start the interactive REPL with an optional player ID:

```bash
cargo run -p mr-roller-cli          # player ID 1 (default)
cargo run -p mr-roller-cli -- 42    # player ID 42
```

By default the CLI loads `mr-roller.toml` from the current directory and uses
in-memory storage unless a database URL is configured:

```toml
[database]
url = "sqlite:mr-roller.db?mode=rwc"
```

You can also override config values with config-rs' nested environment variable
format:

```bash
MR_ROLLER__DATABASE__URL='sqlite:mr-roller.db?mode=rwc' cargo run -p mr-roller-cli
```

To use a different config file, set `MR_ROLLER_CONFIG`:

```bash
MR_ROLLER_CONFIG='./config/local.toml' cargo run -p mr-roller-cli
```

Available commands:

```
/start        — join the game, receive a starter dice
/use <id>     — use an item from your inventory
/inventory    — list your items
/leaderboard  — show top scores
/admin        — show admin-only commands if you are an admin
/quit         — exit
```

Dice rolls are limited by `CooldownConfig`. By default, players can roll once per
UTC day: after a dice roll, they are blocked until either midnight UTC passes or
the configured cooldown duration elapses. Reroll tokens clear the player's roll
cooldown and are consumed when used.

## Admin commands

Players have a persistent `is_admin` flag. To bootstrap the first admins, add
player IDs to `mr-roller.toml` before they run `/start`:

```toml
[admin]
bootstrap_admin_ids = [1, 42]
```

Or override it from the environment:

```bash
MR_ROLLER__ADMIN__BOOTSTRAP_ADMIN_IDS='1,42' cargo run -p mr-roller-cli -- 1
```

Admins can use:

```text
/admin                                      — list admin commands and grantable items
/admin give <player-id> <item>              — give an item to any player ID
/admin set-admin <player-id> <true|false>   — grant or revoke admin status
```

Grantable item keys are `starter_dice`, `regular_dice`, `lucky_dice`,
`cursed_dice`, and `reroll_token`.

## Library usage

```rust
use std::sync::Arc;
use mr_roller::{
    command::{StartCommand, UseItemCommand},
    game::{player::PlayerId, Game},
    store::{
        InMemoryPlayerStore, InMemoryInventoryStore, InMemoryLeaderboardStore,
    },
};

#[tokio::main]
async fn main() {
    let game = Game::new(
        Arc::new(InMemoryPlayerStore::new()),
        Arc::new(InMemoryInventoryStore::new()),
        Arc::new(InMemoryLeaderboardStore::new()),
    );

    // Start a new player
    let resp = game.execute(StartCommand { player_id: PlayerId::new(1) }).await;
    println!("{} {}", resp.kind, resp.message);

    // Give the player a regular dice and use it
    // (in a real app this comes from the inventory store)
    // ...
}
```

## Adding a new item

1. Create a struct implementing `GameItem`:

```rust
#[derive(Debug, Clone)]
pub struct MyDice { pub name: String, pub description: String }

impl GameItem for MyDice {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }
    fn handle(&self) -> Response { /* ... */ }
}
```

2. Add one line to `define_items!` in `mr-roller/src/game/item.rs`:

```rust
define_items! {
    BasicDice(...) as basic_dice,
    MyDice(dice::my_dice::MyDice) as my_dice,  // ← new line
    RerollToken(...) as reroll_token,
}
```

That's it — `From`, accessor methods, and delegate methods are generated automatically.
