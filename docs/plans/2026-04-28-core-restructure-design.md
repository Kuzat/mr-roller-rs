# Mr Roller — Core Library Restructure Design

**Date:** 2026-04-28
**Status:** Design validated, awaiting implementation

## Problem

The current `mr-roller` library couples game logic to storage backends via enums with
match arms (`MrRollerState::LocalState | DatabaseState`), uses a closed `Item` enum
without ergonomic extension points, and has no structured command dispatch. Adding a
new backend, item type, or game command requires touching many files.

## Goals

1. Swappable storage backends (in-memory, SQLite) without touching game logic
2. New item types added in one place (one struct + one macro line)
3. New game commands added without modifying existing handlers
4. Fully async from the start (Discord end goal, CLI as initial test target)
5. Frontends (CLI, Discord) only know about a `Game` dispatcher — zero storage or item
   knowledge

## Design

### 1. Storage Layer — Focused async traits

Three separate traits replace the monolithic `MrRollerState` enum. Each uses
`#[async_trait]`:

```rust
#[async_trait]
pub trait PlayerStore: Send + Sync {
    async fn get(&self, id: PlayerId) -> Result<Player, MrRollerError>;
    async fn insert(&self, player: Player) -> Result<(), MrRollerError>;
    async fn remove(&self, id: PlayerId) -> Result<(), MrRollerError>;
    async fn contains(&self, id: PlayerId) -> Result<bool, MrRollerError>;
    async fn all(&self) -> Result<Vec<Player>, MrRollerError>;
}

#[async_trait]
pub trait InventoryStore: Send + Sync {
    async fn get_item(&self, player_id: PlayerId, item_id: ItemId) -> Result<Item, MrRollerError>;
    async fn add_item(&self, player_id: PlayerId, item: Item) -> Result<ItemId, MrRollerError>;
    async fn remove_item(&self, player_id: PlayerId, item_id: ItemId) -> Result<(), MrRollerError>;
    async fn list_items(&self, player_id: PlayerId) -> Result<Vec<(ItemId, Item)>, MrRollerError>;
}

#[async_trait]
pub trait LeaderboardStore: Send + Sync {
    async fn get_scores(&self, limit: usize) -> Result<Vec<(PlayerId, Score)>, MrRollerError>;
    async fn update_score(&self, player_id: PlayerId, score: Score) -> Result<(), MrRollerError>;
}
```

**Initial implementations:**
- `InMemoryPlayerStore` — `Arc<RwLock<HashMap<PlayerId, Player>>>`
- `InMemoryInventoryStore` — `Arc<RwLock<HashMap<PlayerId, HashMap<ItemId, Item>>>>`
- Later: `SqlitePlayerStore`, etc.

**`Player` becomes a plain data struct** — no embedded inventory. Inventory is managed
separately via `InventoryStore`.

### 2. Command System — Structs + handler trait

Each game action is a struct implementing `Command`:

```rust
#[async_trait]
pub trait Command: Send {
    type Output: Into<Response>;
    async fn execute(self, ctx: &Context) -> Result<Self::Output, MrRollerError>;
}

pub struct Context<'a> {
    pub players: &'a dyn PlayerStore,
    pub inventory: &'a dyn InventoryStore,
    pub leaderboard: &'a dyn LeaderboardStore,
}
```

**Concrete commands:**
- `StartCommand { player_id }` — adds player, grants starter dice
- `RollCommand { player_id, item_id }` — rolls a dice from inventory
- `UseItemCommand { player_id, item_id }` — uses any usable item
- `InventoryCommand { player_id }` — lists player inventory
- `LeaderboardCommand { limit }` — returns top scores
- Future: `ShopCommand`, `EventCommand`, `HelpCommand`

**`Response`** is a flat data type — no game logic, just structured render data:

```rust
pub struct Response {
    pub kind: ResponseKind,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

pub enum ResponseKind {
    Success,
    Error,
    DiceRoll,
    Inventory,
    Leaderboard,
}
```

A top-level `Game` dispatcher wires everything together:

```rust
pub struct Game<P: PlayerStore, I: InventoryStore, L: LeaderboardStore> {
    context: Context,
}

impl<P, I, L> Game<P, I, L> {
    pub async fn execute<C: Command>(&self, cmd: C) -> Response { ... }
}
```

CLI and Discord only interact with `Game::execute(command)`.

### 3. Item System — Macro-powered enum

`Item` stays an enum (critical for DB serialization), but a declarative macro makes
extension one line per item type:

```rust
pub trait GameItem: Debug + Clone + Serialize + DeserializeOwned + Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn handle(&self) -> Response;
}

define_items! {
    BasicDice(basic_dice::BasicDice) as basic_dice,
    RerollToken(reroll_token::RerollToken) as reroll_token,
}
```

The `define_items!` macro generates:
- The `Item` enum with all variants
- `From<$type> for Item` for each variant
- Accessor methods (e.g. `Item::basic_dice() -> Option<BasicDice>`)
- Delegating implementations of `handle()` and `name()`

**Adding a new item type:**
1. Create struct implementing `GameItem`
2. Add one line to `define_items! {}`
3. No other files modified

## Implementation Plan (incremental)

1. **Phase 1 — Storage traits & in-memory impls**
   - Define `PlayerStore`, `InventoryStore`, `LeaderboardStore` traits
   - Implement `InMemoryPlayerStore`, `InMemoryInventoryStore`, `InMemoryLeaderboardStore`
   - Add `async_trait`, `serde`, `tokio` dependencies
   - Extract `Inventory` out of `Player`
   - Port existing tests to async

2. **Phase 2 — Command system & Response types**
   - Define `Command` trait, `Context`, `Response`
   - Implement existing commands (Start, Roll/Use)
   - Build `Game` dispatcher
   - Port existing tests

3. **Phase 3 — Item system macro**
   - Define `GameItem` trait
   - Build `define_items!` macro
   - Port `BasicDice` and `RerollToken`
   - Add serde derives

4. **Phase 4 — New commands & new items**
   - `InventoryCommand`, `LeaderboardCommand`
   - Additional dice/items
   - Backfill missing tests

5. **Phase 5 — CLI fix & integration**
   - Rewrite `mr-roller-cli` against new API
   - Basic CLI loop

6. **Phase 6 — Database backend**
   - `SqlitePlayerStore`, `SqliteInventoryStore`
   - Integration tests against SQLite

## Dependencies to add

- `async-trait` — async trait methods
- `serde` + `serde_json` — `Response` data field, item serialization
- `tokio` — async runtime (full features for initial dev)
