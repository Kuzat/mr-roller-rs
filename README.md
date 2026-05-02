# Mr Roller (Rust)

This repository contains a Rust implementation of the **Mr Roller** dice game. It
is organized as a Cargo workspace with the following crates:

- **mr-roller** — library crate containing all core game logic.
- **mr-roller-cli** — interactive command-line REPL.
- **mr-roller-discord** — Discord slash-command and event bot frontend.
- **mr-roller-rs** — workspace root with a simple example binary.

## Architecture

The library is built around three pluggable layers:

| Layer | Purpose | Swap for... |
|---|---|---|
| **Stores** (`PlayerStore`, `InventoryStore`, `LeaderboardStore`) | Persistence | In-memory → SQLite, PostgreSQL |
| **Commands** (`StartCommand`, `UseItemCommand`, …) | Game actions | Add new commands without touching existing code |
| **Items** (`define_items!` macro) | Dice, tokens, etc. | One struct + one macro line per new item |
| **Cooldowns** (`CooldownConfig`) | Roll limits | Default: 24h cooldown, reset at midnight UTC |
| **Events** (`EventStore`, event commands) | Global random events | Core logic is frontend-agnostic |
| **Migrations** (`mr-roller/migrations`) | SQLite schema changes | Managed by `sqlx::migrate!` |

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

# build the Discord bot
cargo build -p mr-roller-discord
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
/shop         — list buyable dice
/buy <item>   — buy an item from the shop
/leaderboard  — show top scores
/events       — list active global events
/event claim <id> — claim an active event
/event trash <id> — trash an active event
/admin        — show admin-only commands if you are an admin
/quit         — exit
```

Dice rolls are limited by `CooldownConfig`. By default, players can roll once per
UTC day: after a dice roll, they are blocked until either midnight UTC passes or
the configured cooldown duration elapses. Dice rolls award XP and coins equal to
the roll result. Reroll tokens clear the player's roll cooldown and are consumed
when used.

## Shop

The shop sells dice for coins:

```text
/shop        — list buyable dice and prices
/buy <item>  — buy a shop item
```

Current shop item keys are `starter_dice`, `regular_dice`, `lucky_dice`, and
`cursed_dice`. Reroll tokens are not sold in the shop yet.

## Events

Events are global and frontend-agnostic: the core library stores event state and
exposes claim/trash commands, while a Discord or CLI frontend decides how to
render buttons/messages. The first implemented event is Random Item Spawn.

```text
/events            — list active events
/event claim <id>  — claim an event reward
/event trash <id>  — trash an active event
```

Admins can manually spawn the event for testing:

```text
/admin event spawn-random-item
```

Random item spawn is configurable in `mr-roller.toml`:

```toml
[events]
enabled = true
check_interval_seconds = 60
spawn_chance_per_check = 0.004
max_active_events = 1

[events.random_item_spawn]
enabled = true
timeout_seconds = 900

[[events.random_item_spawn.items]]
kind = "regular_dice"
weight = 5
```

The CLI starts a background event scheduler when events are enabled. It checks
for random item spawns every `check_interval_seconds` and prints spawned events
to the terminal. A Discord/server runtime can reuse the same `EventScheduler`
and publish spawned event responses as Discord messages/buttons.

## Discord bot

The Discord frontend is a long-lived public application process built with
`poise` and `serenity`. It uses Discord user snowflakes as `PlayerId` values,
stores game state in PostgreSQL, registers slash commands, renders structured
game responses as embeds, and posts random item events with claim/trash buttons.

The Discord process is multi-tenant. A server manager installs the app, runs
`/setup channel:#dice-game`, and the bot creates an isolated game for that
Discord guild + channel. The same Discord server can host separate games in
separate channels.

Use the Discord-specific config file, which includes the PostgreSQL database URL
from `compose.yaml`:

```bash
docker compose up -d postgres

MR_ROLLER_CONFIG='./mr-roller-discord.toml' \
MR_ROLLER__DISCORD__TOKEN='your-bot-token' \
cargo run -p mr-roller-discord
```

See [docs/discord.md](docs/discord.md) for the full setup guide, including
Discord application setup, bot invite permissions, multi-tenant `/setup`, and
available commands.

The Discord binary requires a PostgreSQL `database.url`. The multi-tenant config
registers slash commands globally so the same bot install works across many
servers. SQLite remains available for the core crate, tests, CLI, and local
single-process use, but not for the public Discord runtime. Migrations are
applied automatically when the Discord process starts.

## Database migrations

SQLite schema changes are tracked in `mr-roller/migrations` and applied automatically
when `SqliteStore` connects. Add future schema changes as new timestamped `.sql`
files in that directory instead of editing existing migrations.

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
/admin coins <player-id> <amount>           — add or remove coins, e.g. 50 or -10
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
