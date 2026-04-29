# Mr Roller Architecture

This document describes how the `mr-roller-rs` workspace is organized, which architectural patterns are used, and how to extend each subsystem safely.

## Workspace layout

```text
mr-roller-rs/
├── mr-roller/       # Core game logic library
├── mr-roller-cli/   # CLI frontend / local testing interface
├── src/             # Root example binary
├── docs/            # Architecture and planning docs
└── todo.md          # Open tasks and roadmap
```

The main design goal is to keep **core game rules independent from frontends and persistence**. Discord, CLI, tests, and future frontends should all call the same library API.

## High-level architecture

```text
Frontend (CLI, Discord, tests)
        │
        ▼
Game dispatcher
        │
        ▼
Command system
        │
        ├── PlayerStore
        ├── InventoryStore
        ├── LeaderboardStore
        └── CooldownConfig
        │
        ▼
Domain model: Player, Item, Dice, Tokens, Response
```

The library is organized around four main patterns:

1. **Command pattern** for game actions
2. **Repository / store traits** for persistence
3. **Macro-generated enum registry** for items
4. **Structured response DTOs** for frontend rendering

---

## Core crate: `mr-roller`

The core crate contains all game rules and domain logic. Frontends should not duplicate game decisions such as cooldown checks, item behavior, scoring, or inventory changes.

Important modules:

```text
mr-roller/src/
├── command.rs       # Game commands and command handler trait
├── cooldown.rs      # Roll cooldown configuration and rules
├── errors.rs        # Domain/storage error type
├── game.rs          # Game dispatcher and domain module exports
├── response.rs      # Frontend-facing response objects
├── game/
│   ├── inventory.rs # Shared ItemId type / legacy local inventory helper
│   ├── item.rs      # GameItem trait, Item enum macro, item registry
│   ├── item/dice/   # Dice item implementations
│   ├── item/tokens/ # Token item implementations
│   └── player.rs    # Player and PlayerId domain types
└── store/
    ├── player.rs    # PlayerStore trait + in-memory implementation
    ├── inventory.rs # InventoryStore trait + in-memory implementation
    ├── leaderboard.rs # LeaderboardStore trait + in-memory implementation
    └── sqlite.rs    # SQLite-backed store implementation
```

---

## Pattern 1: Command pattern

Game actions are represented as command structs implementing the async `Command` trait.

Examples:

- `StartCommand`
- `UseItemCommand`
- `InventoryCommand`
- `LeaderboardCommand`
- Admin or future commands can follow the same pattern

Each command owns its input data and executes against a shared `Context`:

```rust
#[async_trait]
pub trait Command: Send {
    type Output: Into<Response>;
    async fn execute(self, ctx: &Context<'_>) -> Result<Self::Output, MrRollerError>;
}
```

`Context` provides access to dependencies:

```rust
pub struct Context<'a> {
    pub players: &'a dyn PlayerStore,
    pub inventory: &'a dyn InventoryStore,
    pub leaderboard: &'a dyn LeaderboardStore,
    pub cooldown: &'a CooldownConfig,
}
```

### Why this pattern?

- New game actions can be added without changing existing commands.
- Commands are easy to unit test because stores are trait-based.
- CLI and Discord can map user input to command structs and let the core library handle rules.
- Commands become the boundary where cross-cutting rules happen, such as cooldown checks, inventory updates, and score updates.

### Adding a new command

1. Add a command struct in `command.rs` or a future `command/` submodule.
2. Implement `Command`.
3. Use stores through `Context`.
4. Return a `Response`.
5. Teach each frontend how to parse user input into the command.

Example shape:

```rust
pub struct ShopCommand {
    pub player_id: PlayerId,
}

#[async_trait]
impl Command for ShopCommand {
    type Output = Response;

    async fn execute(self, ctx: &Context<'_>) -> Result<Response, MrRollerError> {
        let player = ctx.players.get(self.player_id).await?;
        // game logic here
        Ok(Response::success("Shop opened."))
    }
}
```

---

## Pattern 2: Store traits / repository pattern

Persistence is abstracted behind focused async traits:

- `PlayerStore`
- `InventoryStore`
- `LeaderboardStore`

The game logic only depends on these traits, not on concrete backends.

Current implementations:

| Trait | In-memory implementation | SQLite implementation |
|---|---|---|
| `PlayerStore` | `InMemoryPlayerStore` | `SqliteStore` |
| `InventoryStore` | `InMemoryInventoryStore` | `SqliteStore` |
| `LeaderboardStore` | `InMemoryLeaderboardStore` | `SqliteStore` |

`SqliteStore` implements all three traits so the whole game can be backed by one database connection.

### Why this pattern?

- CLI and tests can use in-memory storage.
- Discord or production deployments can use SQLite now, and another DB later.
- Game logic does not need to know whether data is local, SQLite, or remote.
- Store implementations can be tested independently.

### Store responsibilities

Stores should only persist and retrieve data. They should not make gameplay decisions.

Good store behavior:

- Insert/get/update/remove players
- Add/list/remove inventory items
- Update/get leaderboard scores
- Convert DB errors into `MrRollerError`
- Run schema setup or migrations if needed

Avoid putting these in stores:

- Cooldown rules
- Dice roll logic
- Item behavior
- Command permission decisions
- Frontend formatting

---

## Pattern 3: Game dispatcher

`Game` is the frontend-facing entry point.

```rust
pub struct Game {
    players: Arc<dyn PlayerStore>,
    inventory: Arc<dyn InventoryStore>,
    leaderboard: Arc<dyn LeaderboardStore>,
    cooldown: CooldownConfig,
}
```

Frontends call:

```rust
game.execute(command).await
```

`Game` builds a `Context`, runs the command, and converts errors into renderable `Response` objects.

### Why this pattern?

- Frontends do not need direct access to stores.
- Error handling is centralized.
- Swapping backends only changes how `Game` is constructed.
- The same command works in CLI, Discord, and tests.

---

## Pattern 4: Structured responses

Commands return `Response` objects instead of strings.

```rust
pub struct Response {
    pub kind: ResponseKind,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
```

`ResponseKind` tells frontends how to render the response:

- `Success`
- `Error`
- `DiceRoll`
- `Inventory`
- `Leaderboard`

### Why this pattern?

Different frontends need different rendering:

- CLI prints text.
- Discord may use embeds, colors, buttons, or autocomplete.
- Tests can inspect structured fields.

The core library gives structured intent; frontends decide presentation.

---

## Pattern 5: Macro-generated item registry

Items are represented as a serializable enum generated by `define_items!`.

```rust
define_items! {
    BasicDice(dice::basic_dice::BasicDice) as basic_dice,
    LuckyDice(dice::lucky_dice::LuckyDice) as lucky_dice,
    CursedDice(dice::cursed_dice::CursedDice) as cursed_dice,
    RerollToken(tokens::reroll_token::RerollToken) as reroll_token,
}
```

Each concrete item implements `GameItem`:

```rust
pub trait GameItem: Debug + Clone + Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn handle(&self) -> Response;
    fn consumes_daily_roll(&self) -> bool { true }
}
```

The macro generates:

- `enum Item`
- `From<T> for Item`
- Accessor helpers, e.g. `item.basic_dice()`
- Delegating methods:
  - `item.name()`
  - `item.description()`
  - `item.handle()`
  - `item.consumes_daily_roll()`
- Serde support for persistence

### Why an enum instead of trait objects?

The project uses an enum because it is easier to serialize and persist. SQLite stores inventory items as JSON encoded `Item` values. Trait objects would make persistence harder because dynamic types need custom type registries and deserialization logic.

### Adding a new item

1. Create a new item struct.
2. Implement `GameItem`.
3. Add the module export.
4. Add one line to `define_items!`.
5. Add tests.

Example:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FireDice {
    pub name: String,
    pub description: String,
}

impl GameItem for FireDice {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }
    fn handle(&self) -> Response {
        Response::dice_roll("Fire dice rolled!", 6)
    }
}
```

Then register it:

```rust
define_items! {
    BasicDice(dice::basic_dice::BasicDice) as basic_dice,
    FireDice(dice::fire_dice::FireDice) as fire_dice,
    RerollToken(tokens::reroll_token::RerollToken) as reroll_token,
}
```

---

## Cooldown system

Roll limiting is handled by `CooldownConfig` plus `Player::last_roll_at`.

Default behavior:

- 24 hour cooldown
- Reset at midnight UTC

A player may roll again when **either**:

1. Midnight UTC has passed since their last roll, or
2. The configured duration has elapsed

This supports both normal daily play and limited-time events.

Examples:

| Config | Last roll | Next allowed |
|---|---:|---:|
| 24h + midnight reset | 13:00 Day 1 | 00:00 Day 2 |
| 12h, no midnight reset | 13:00 Day 1 | 01:00 Day 2 |
| 12h + midnight reset | 02:00 Day 1 | 14:00 Day 1 |

### Reroll tokens

Reroll tokens are modeled as items that do **not** consume the daily roll:

```rust
fn consumes_daily_roll(&self) -> bool { false }
```

When a reroll token is used:

1. The player's `last_roll_at` is cleared.
2. The token is removed from inventory.
3. The next dice roll is allowed.
4. That dice roll sets `last_roll_at` again.

---

## SQLite persistence

SQLite support lives in `store/sqlite.rs`.

`SqliteStore` owns a `SqlitePool` and implements all current store traits.

### Schema ownership

`SqliteStore::migrate()` creates the schema if missing. At the moment this is lightweight application-managed migration using `CREATE TABLE IF NOT EXISTS` and simple compatibility changes.

Current tables:

- `players`
- `inventory_items`
- `leaderboard_scores`

Inventory items are stored as JSON:

```text
inventory_items.item_json TEXT NOT NULL
```

This keeps item persistence aligned with the macro-generated `Item` enum.

### CLI SQLite usage

The CLI defaults to in-memory storage. Set `MR_ROLLER_DB_URL` to use SQLite:

```bash
MR_ROLLER_DB_URL='sqlite:mr-roller.db?mode=rwc' cargo run -p mr-roller-cli
```

---

## Frontend responsibilities

Frontends should be thin adapters.

A frontend should:

- Parse user input
- Construct command structs
- Call `Game::execute(command).await`
- Render `Response`

A frontend should not:

- Enforce cooldown rules
- Modify stores directly for normal gameplay
- Roll dice itself
- Decide scoring rules
- Duplicate item behavior

This keeps CLI and Discord consistent.

---

## Testing strategy

The architecture supports layered testing:

### Domain tests

Test item behavior, cooldown rules, and player model behavior directly.

### Store tests

Test each store implementation against the same expected semantics:

- In-memory stores
- SQLite store

### Command tests

Test commands against in-memory stores for fast feedback.

### Frontend tests

CLI/Discord tests should mostly verify parsing and rendering, not game rules.

---

## Extension guidelines

### Add a new dice or token

Use the `GameItem` + `define_items!` flow.

### Add a new gameplay action

Use the `Command` pattern.

### Add a new database backend

Implement the existing store traits:

- `PlayerStore`
- `InventoryStore`
- `LeaderboardStore`

Then construct `Game` with the new store implementation.

### Add a new frontend

Only depend on:

- `Game`
- command structs
- domain IDs like `PlayerId` and `ItemId`
- `Response`

Avoid direct store access unless building admin tooling or diagnostics.

---

## Design principles

1. **Core game rules live in `mr-roller`**
2. **Frontends are adapters, not rule engines**
3. **Storage is abstracted behind focused traits**
4. **Commands are the unit of gameplay behavior**
5. **Items are enum-backed for serialization**
6. **Responses are structured and frontend-neutral**
7. **Prefer small extension points over large rewrites**
