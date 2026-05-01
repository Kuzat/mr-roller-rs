# Public Multi-Tenant Discord App Plan

Date: 2026-04-29

## Goal

Turn Mr Roller from a single-server Discord bot into a hosted Discord application that any server owner can install with an OAuth2 install link. After installation, a server manager can run a setup command, choose a text channel, and start an isolated Mr Roller game for that channel.

Initial target setup flow:

```text
1. User clicks Discord install link and adds Mr Roller to their server.
2. User runs `/setup channel:#dice-game`.
3. Bot creates a game scoped to that Discord guild + channel.
4. Bot makes the setup user the first admin for that game.
5. Bot posts a welcome message in the selected channel.
6. Players use `/start`, `/inventory`, `/shop`, etc. in that channel.
```

A single Discord server may eventually host multiple games in different channels. Each game must have isolated players, inventory, leaderboard, events, and admins.

## Key design decision

Use durable multi-tenant storage with `game_id` on all persisted game data.

Recommended approach:

```text
Discord guild + channel -> discord_games row -> game_id

players             scoped by game_id
inventory_items     scoped by game_id
leaderboard_scores  scoped by game_id
active_events       scoped by game_id
```

This is more durable than one database per game and works better for backups, migrations, hosting, analytics, and operations.

## Database choice

Switch the production/public Discord runtime from SQLite to PostgreSQL.

Reasons:

- Better fit for hosted, multi-tenant, always-on service.
- Safer concurrent access model than SQLite for many guilds/channels.
- Easier to run on common hosts: Fly.io, Railway, Render, Supabase, Neon, RDS, etc.
- Better migration and operational story as tenant count grows.
- Enables future dashboards, analytics, admin tools, and support queries.

SQLite can remain useful for:

- Local development.
- CLI runtime.
- Core crate tests.
- Single-server self-hosted mode if desired.

The public Discord bot should require PostgreSQL.

## Keeping core gameplay pure

We should keep Discord-specific logic out of `mr-roller` as much as possible.

The current core design already helps because `Game` depends on store traits:

```rust
Game::with_event_store(
    players: Arc<dyn PlayerStore>,
    inventory: Arc<dyn InventoryStore>,
    leaderboard: Arc<dyn LeaderboardStore>,
    events: Arc<dyn EventStore>,
    bootstrap_admin_ids,
    event_config,
)
```

The Discord crate can provide a PostgreSQL-backed, game-scoped store implementation:

```rust
pub struct PostgresGameStore {
    pool: PgPool,
    game_id: GameId,
}
```

Then implement the existing core traits inside `mr-roller-discord`:

```rust
impl PlayerStore for PostgresGameStore { ... }
impl InventoryStore for PostgresGameStore { ... }
impl LeaderboardStore for PostgresGameStore { ... }
impl EventStore for PostgresGameStore { ... }
```

Every query includes `WHERE game_id = $1`, but the core game module does not need to know about Discord guilds, channels, installs, or tenants.

This means Option A does **not** require changing core gameplay rules. It requires either:

1. A new Discord-specific Postgres store in `mr-roller-discord`, or
2. A generic Postgres store in `mr-roller` that supports scoped game IDs.

Recommendation for first public Discord iteration: implement the multi-tenant Postgres store in `mr-roller-discord`. Keep the core crate focused on gameplay traits, commands, items, and responses.

Potential small core changes that may still be useful:

- Expose helper DTOs if Discord autocomplete needs cleaner reads.
- Add small trait bounds or constructors if needed.
- Keep `GameId` out of the core command APIs unless another frontend also needs multi-tenancy.

## Data model

### `discord_games`

Stores configured game channels.

```sql
CREATE TABLE discord_games (
    game_id UUID PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    created_by_user_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    events_enabled BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (guild_id, channel_id)
);
```

Notes:

- `guild_id + channel_id` identifies a Discord game location.
- `game_id` is the internal stable tenant key.
- `created_by_user_id` is the setup user and initial admin.

### `players`

Game-scoped players.

```sql
CREATE TABLE players (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    id BIGINT NOT NULL,
    last_roll_at TIMESTAMPTZ,
    luck BIGINT NOT NULL DEFAULT 0,
    coins BIGINT NOT NULL DEFAULT 0,
    xp BIGINT NOT NULL DEFAULT 0,
    is_admin BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (game_id, id)
);
```

The same Discord user can have different state in different game channels.

### `inventory_items`

```sql
CREATE TABLE inventory_items (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    item_id UUID NOT NULL,
    player_id BIGINT NOT NULL,
    item_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (game_id, item_id),
    FOREIGN KEY (game_id, player_id) REFERENCES players(game_id, id) ON DELETE CASCADE
);
```

### `leaderboard_scores`

```sql
CREATE TABLE leaderboard_scores (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    player_id BIGINT NOT NULL,
    xp BIGINT NOT NULL DEFAULT 0,
    coins BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (game_id, player_id),
    FOREIGN KEY (game_id, player_id) REFERENCES players(game_id, id) ON DELETE CASCADE
);
```

### `active_events`

```sql
CREATE TABLE active_events (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    id UUID NOT NULL,
    kind TEXT NOT NULL,
    event_json JSONB NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (game_id, id)
);
```

### Optional later: `discord_event_messages`

For restart rehydration and message cleanup.

```sql
CREATE TABLE discord_event_messages (
    game_id UUID NOT NULL REFERENCES discord_games(game_id) ON DELETE CASCADE,
    event_id UUID NOT NULL,
    channel_id BIGINT NOT NULL,
    message_id BIGINT NOT NULL,
    PRIMARY KEY (game_id, event_id)
);
```

Not required for the first public version.

## Discord configuration changes

Current config is single-channel:

```toml
[discord]
home_channel_id = 123
```

Public app config should become service-level:

```toml
[discord]
token = ""
guild_id = 0 # optional dev guild only

[database]
url = "postgres://..."
```

Remove production dependency on `discord.home_channel_id`. Game channels come from `discord_games` rows created by `/setup`.

For local development, `guild_id` remains useful for instant slash command registration.

For production, omit `guild_id` or set it to `0` so commands are registered globally.

## Discord install flow

### Developer Portal setup

OAuth2 scopes:

```text
bot
applications.commands
```

Bot permissions:

```text
View Channels
Send Messages
Embed Links
Read Message History
Use Slash Commands
```

Potential future permissions:

```text
Manage Messages    # only if we add cleanup/moderation behavior
```

### Install URL

Generate an OAuth2 URL from the Discord Developer Portal or document one in the README.

The user installs the app to a server, then runs setup manually.

## Setup command

Add:

```text
/setup channel:<text-channel>
```

Behavior:

1. Require the command to be run inside a guild.
2. Require the caller to have a suitable Discord permission:
   - `Administrator`, or
   - `Manage Guild`, or
   - `Manage Channels`.
3. Validate the bot can send messages and embeds in the selected channel.
4. Create `discord_games` row for `guild_id + channel_id` if missing.
5. Create or update player row for the setup user in that game.
6. Set setup user `is_admin = true` for that game.
7. Reply privately to setup user:

```text
Mr Roller is ready in #dice-game.
You are now an admin for this game.
```

8. Post publicly in selected channel:

```text
🎲 Mr Roller has started a new game in this channel!
Run /start to join.
```

If a game already exists for that channel:

- Reply privately with current status.
- If caller has permission, optionally make them admin too or tell them who created it.

Initial recommendation: if the caller has setup permissions and the game already exists, add them as admin and report that the game already existed.

## Command routing

Every gameplay command should resolve the game from the current Discord channel.

Current model:

```rust
ctx.data().game.execute(command).await
```

New model:

```rust
let game = ctx.data().games.game_for_context(ctx).await?;
game.execute(command).await
```

If no game exists for the current channel, reply ephemerally:

```text
No Mr Roller game is configured for this channel.
Ask a server manager to run `/setup channel:#this-channel`.
```

Commands affected:

```text
/start
/inventory
/shop
/buy
/leaderboard
/use
/events
/event claim
/event trash
/admin give
/admin coins
/admin set-admin
/admin event spawn-random-item
```

Admin checks remain in the core command system via game-scoped `players.is_admin`.

## Game registry / resolver

Add a Discord-side service:

```rust
pub struct DiscordGameRegistry {
    pool: PgPool,
    event_config: EventsConfig,
}
```

Responsibilities:

- Look up `discord_games` by guild/channel.
- Create game rows during setup.
- Build game-scoped stores.
- Return `Arc<Game>` for a game.
- Optionally cache `Arc<Game>` by `game_id`.

Example API:

```rust
impl DiscordGameRegistry {
    async fn setup_game(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        created_by: UserId,
    ) -> Result<DiscordGame>;

    async fn game_for_channel(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Result<Option<Arc<Game>>>;

    async fn list_games(&self) -> Result<Vec<DiscordGame>>;
}
```

Implementation detail:

```rust
let store = Arc::new(PostgresGameStore::new(pool.clone(), game_id));
let game = Arc::new(Game::with_event_store(
    store.clone(),
    store.clone(),
    store.clone(),
    store.clone(),
    Vec::new(),
    event_config.clone(),
));
```

No Discord-specific data enters `Game`.

## PostgreSQL store implementation

Add module in `mr-roller-discord`:

```text
mr-roller-discord/src/storage/
├── mod.rs
├── migrations.rs or migrations/
├── postgres.rs
└── registry.rs
```

`PostgresGameStore` implements core store traits.

Important query rules:

- Every `SELECT`, `INSERT`, `UPDATE`, and `DELETE` includes `game_id`.
- Player IDs remain Discord snowflakes as `u64` in Rust and `BIGINT` in Postgres.
- Items and events are serialized as JSONB.
- Use transactions when setup creates game + admin player.

Potential ID caveat:

Discord snowflakes fit in signed 64-bit today, but are conceptually unsigned. Existing SQLite code casts `u64` to `i64`. For Postgres we can use `BIGINT` with careful conversion and validation, or store snowflakes as `TEXT` to avoid signedness concerns.

Recommendation:

- For least surprise, store Discord IDs as `TEXT` in Discord metadata tables.
- For compatibility with current `PlayerId(u64)`, either:
  - Store player IDs as `BIGINT` with checked conversion, or
  - Store player IDs as `TEXT` and parse to/from `u64`.

Most durable option: store Discord snowflakes as `TEXT` in all Discord-owned Postgres tables.

If we use `TEXT`, tables become:

```sql
guild_id TEXT NOT NULL
channel_id TEXT NOT NULL
player_id TEXT NOT NULL
created_by_user_id TEXT NOT NULL
```

This avoids signed integer edge cases.

## Scheduler changes

Current scheduler runs for one configured home channel.

New scheduler:

```text
Every check interval:
  load all configured discord_games where events_enabled = true
  for each game:
    build/resolve scoped Game
    run EventScheduler::tick(game)
    if event spawned:
      post event embed/buttons to that game's channel
```

Use one scheduler task that loops over configured games. Avoid one infinite task per game in the first version.

Pseudo-code:

```rust
loop every check_interval:
    let games = registry.list_games_with_events_enabled().await?;
    for discord_game in games {
        let game = registry.game_for_id(discord_game.game_id).await?;
        if let Some(response) = scheduler.tick(&game).await {
            publish_event_response(http, discord_game.channel_id, &response).await?;
        }
    }
```

This keeps event logic core-agnostic and Discord routing in the Discord crate.

## Admin model

Initial version:

- Setup user becomes `is_admin = true` in the game-scoped `players` row.
- Existing admin commands continue to use core admin checks.
- Admins are per game because players are per `game_id`.

Later improvements:

```text
/admin add user:<user>
/admin remove user:<user>
/admin list
```

Role-based admin can come later:

```text
/settings admin-role role:<role>
```

Do not add role-based admin in the first public version unless needed.

## User-facing commands

### Setup and status

```text
/setup channel:<text-channel>
/status
```

`/status` in a game channel should show:

- Game configured or not.
- Channel.
- Events enabled.
- Number of players, if easy.

### Player commands

Same as current:

```text
/start
/inventory
/shop
/buy
/leaderboard
/use
/events
/event claim
/event trash
```

### Admin commands

Same as current initially:

```text
/admin give
/admin coins
/admin set-admin
/admin event spawn-random-item
```

Potentially add later:

```text
/admin setup-info
/admin disable-events
/admin enable-events
/admin delete-game
```

## Error handling and UX

### No game in channel

Ephemeral response:

```text
No Mr Roller game is configured for this channel.
Ask a server manager to run `/setup channel:#this-channel`.
```

### Command used in DM

```text
Mr Roller games must be used inside a server channel.
```

### Missing bot permissions

During setup:

```text
I cannot send messages and embeds in #dice-game yet.
Please give me access to that channel and try again.
```

### Duplicate setup

```text
A Mr Roller game already exists in #dice-game.
You have been added as an admin for that game.
```

## Migration strategy from current bot

Current bot has SQLite single-channel state. For public multi-tenant launch, simplest path is:

1. Keep SQLite support for local/dev/self-hosted single bot.
2. Add Postgres support to Discord crate.
3. Public deployment starts fresh in Postgres.
4. Optional later migration tool:

```text
sqlite single-channel DB + guild/channel config -> Postgres discord_game
```

Do not block public architecture on migration tooling unless current production data must be preserved.

## Implementation phases

### Phase 1 — Public app planning and config

- Add this plan.
- Decide hosting provider and Postgres provider.
- Decide final Discord permissions for install URL.
- Update Discord config to support Postgres public mode.

Acceptance:

- Plan and config direction are agreed.

### Phase 2 — Postgres storage in Discord crate

- Add `sqlx` Postgres dependency to `mr-roller-discord`.
- Add Discord crate migrations for multi-tenant schema.
- Implement `PostgresGameStore` for:
  - `PlayerStore`
  - `InventoryStore`
  - `LeaderboardStore`
  - `EventStore`
- Add tests for scoped isolation between two `game_id`s.

Acceptance:

- Same `player_id` can exist in two games with different coins/inventory.
- Store trait tests pass against Postgres, ideally using testcontainers or a configured test DB.

### Phase 3 — Discord game registry

- Add `DiscordGameRegistry`.
- Add `discord_games` setup/query helpers.
- Add game resolver for guild/channel.
- Add optional in-memory cache of `Arc<Game>` by `game_id`.

Acceptance:

- Can create a game for guild/channel.
- Can resolve a scoped `Game` for guild/channel.
- Missing game returns a clean no-game result.

### Phase 4 — `/setup` command

- Implement `/setup channel:<text-channel>`.
- Check caller permissions.
- Validate bot access to selected channel.
- Create game row.
- Add setup user as admin.
- Reply privately and post channel welcome.

Acceptance:

- Installing bot + running `/setup` creates a playable channel game.
- Setup user can run admin commands in that game.

### Phase 5 — Route existing commands by channel

- Replace global `Arc<Game>` usage with registry resolution.
- Add consistent no-game ephemeral errors.
- Keep `/inventory` and `/shop` private.
- Keep admin responses private by default.

Acceptance:

- Two channels can run independent games.
- Same user can have different inventory/coins in each channel.

### Phase 6 — Multi-game event scheduler

- Replace single-channel scheduler with registry-backed scheduler.
- Iterate all configured games with events enabled.
- Post events to each game's channel.

Acceptance:

- Events spawn in the correct configured channel.
- Events do not leak across games.

### Phase 7 — Production install and hosting polish

- Register global slash commands in production.
- Document OAuth2 install link.
- Add deployment docs.
- Add health checks/logging.
- Add privacy/TOS docs if preparing for public listing or verification.

Acceptance:

- A new server can install the bot with a link and self-serve setup.

## Testing strategy

### Unit/integration tests

- Postgres store isolation by `game_id`.
- Setup creates game + admin player transactionally.
- Command resolver returns no-game error for unconfigured channels.
- Scheduler only checks configured games.
- Admin permission checks remain per game.

### Manual smoke tests

1. Install bot into dev server.
2. Run `/setup channel:#game-a`.
3. Run `/setup channel:#game-b`.
4. Same user runs `/start` in both channels.
5. Give coins in #game-a.
6. Verify #game-b coins remain unchanged.
7. Buy/use item in #game-a.
8. Verify inventory is isolated.
9. Spawn event in #game-a.
10. Verify event does not appear in #game-b.
11. Let scheduler run and verify events post to configured channels.

## Open decisions

- Whether to allow multiple games per Discord server in the first public release.
  - Recommendation: yes, because the model naturally supports guild + channel.
- Whether setup should require `Manage Guild`, `Manage Channels`, or `Administrator`.
  - Recommendation: allow `Administrator` or `Manage Guild`; consider `Manage Channels` too.
- Whether Discord snowflakes should be stored as `TEXT` or `BIGINT` in Postgres.
  - Recommendation: `TEXT` for Discord-owned IDs to avoid signedness issues.
- Whether Postgres store belongs in `mr-roller-discord` or `mr-roller`.
  - Recommendation: start in `mr-roller-discord` to keep the core crate pure.
- Whether existing SQLite Discord bot mode should remain.
  - Recommendation: keep SQLite for local/self-hosted mode, but require Postgres for public multi-tenant deployment.

## Summary

The public installable Discord version should be built as a multi-tenant hosted app. Discord-specific onboarding, guild/channel mapping, setup commands, and Postgres storage should live in `mr-roller-discord`. The core `mr-roller` crate can remain mostly unchanged because its existing store trait architecture lets the Discord crate provide game-scoped store implementations.

The largest implementation task is not Discord OAuth itself; it is reliable game scoping. Once every store query is scoped by `game_id`, the rest of the bot can route commands by guild/channel and safely support many servers and channels from one hosted process.
