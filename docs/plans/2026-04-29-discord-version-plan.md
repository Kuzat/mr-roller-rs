# Discord Version Plan

Date: 2026-04-29

## Goal

Build a Discord frontend for Mr Roller that runs as a long-lived server process and uses native Discord interactions:

- Slash commands for normal player actions.
- Autocomplete for inventory item selection and shop/event IDs.
- Buttons for event actions such as claiming or trashing a random item spawn.
- Embeds for inventory, shop, item rewards, leaderboard, and event announcements.
- A background event scheduler that posts random events to a configured Discord channel 24/7.

The Discord crate must remain a frontend adapter. Core game rules stay in `mr-roller` and are accessed through `Game::execute(command)`.

## Recommended Discord stack

Use `poise` on top of `serenity`.

Reasons:

- `serenity` is the standard mature Discord Rust library.
- `poise` makes slash commands, typed command parameters, autocomplete, and framework setup simpler.
- It still exposes Serenity types for lower-level features like buttons, embeds, message IDs, and interaction handling.
- It fits the existing command-driven game architecture well.

Proposed crate:

```text
mr-roller-discord/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── config.rs              # Discord-specific config helpers if needed
    ├── bot.rs                 # framework/client setup
    ├── commands/
    │   ├── mod.rs
    │   ├── player.rs          # /start, /inventory, /use, /shop, /buy, /leaderboard
    │   ├── admin.rs           # /admin ...
    │   └── events.rs          # /events, event admin commands
    ├── render/
    │   ├── mod.rs
    │   ├── embeds.rs          # Response -> Discord embed helpers
    │   └── components.rs      # buttons/select menus
    └── events/
        ├── mod.rs
        └── scheduler.rs       # Discord publisher around EventScheduler
```

## Configuration

Extend `mr-roller.toml` with a Discord section:

```toml
[discord]
enabled = true
token = ""                    # prefer env override in production
guild_id = 0                   # optional dev guild for fast command registration
home_channel_id = 0            # channel where random events are posted
```

Production secret override:

```bash
MR_ROLLER__DISCORD__TOKEN='...' cargo run -p mr-roller-discord
```

The Discord binary should require:

- `discord.token`
- `discord.home_channel_id`

For local testing, `guild_id` should be optional but recommended. If set, register slash commands to that guild for instant updates. Global registration can be added later.

## Runtime architecture

The Discord process owns one shared `Arc<Game>`:

```text
Discord gateway/client
        │
        ├── slash commands ───────▶ Game::execute(...)
        │                            │
        │                            ▼
        │                       Response DTO
        │                            │
        ▼                            ▼
Discord renderer ◀────────── embeds/messages/components

Background EventScheduler ─▶ MaybeSpawnRandomItemEventCommand
        │
        ▼
Post event embed + claim/trash buttons to configured channel
```

Use SQLite in Discord runtime. In-memory storage is only useful for local tests and should probably be rejected or warned about when running the Discord server.

## Discord player identity

Map Discord user IDs directly to `PlayerId`:

```rust
PlayerId::new(ctx.author().id.get())
```

This keeps identity stable and avoids a separate account linking table. If a user runs `/start`, they are inserted using their Discord snowflake as the player ID.

## Slash commands

### Player commands

```text
/start
/inventory
/use item:<autocomplete item id>
/shop
/buy item:<autocomplete shop key>
/leaderboard
/events
/event claim event:<autocomplete event id>
/event trash event:<autocomplete event id>
```

Notes:

- `/use` should autocomplete from the player's current inventory.
- Autocomplete display should include item name and short item ID prefix.
- The selected value should be the full `ItemId` UUID.
- `/buy` should autocomplete shop keys.
- `/event claim` and `/event trash` are fallback slash commands; buttons are the primary UX.

### Admin commands

Use slash command groups:

```text
/admin give user:<Discord user> item:<autocomplete admin item>
/admin coins user:<Discord user> amount:<integer>
/admin event spawn-random-item
/admin set-admin user:<Discord user> is-admin:<bool>
```

Admin commands should call the existing admin command structs. Admin identity is the Discord author ID.

## Rendering strategy

Keep rendering outside `mr-roller`. Discord renderer converts `Response` into one of:

- Plain ephemeral response for errors and confirmations.
- Embed for inventory/shop/leaderboard/event details.
- Embed + buttons for spawned events.

Recommended embeds:

### Inventory embed

- Title: `🎒 Inventory`
- Fields: item name, item ID, description
- Include instruction: `Use /use item:<item>`

### Shop embed

- Title: `🛒 Shop`
- Fields: item key, price, description
- Include instruction: `Use /buy item:<item>`

### Random item spawn embed

- Title: `🎉 Random Item Spawn`
- Description from event response.
- Field: event ID, item name, expires at.
- Buttons:
  - `Claim` with custom ID: `event:claim:<event_id>`
  - `Trash` with custom ID: `event:trash:<event_id>`

Button handlers should parse the custom ID, map the Discord user to `PlayerId`, run `ClaimEventCommand` or `TrashEventCommand`, then edit the original message to show final status and remove buttons.

## Event scheduler and Discord publisher

Use the existing `EventScheduler` from `mr-roller`.

Discord-specific publisher:

```rust
scheduler.run(game.clone(), |response| async move {
    // render response as event embed
    // send to configured home_channel_id
    // attach claim/trash buttons
});
```

The core scheduler only returns spawned event responses. The Discord layer is responsible for:

- Sending the message to the configured channel.
- Adding buttons.
- Editing/removing buttons when claimed/trashed.

Future improvement: add a Discord adapter table mapping `event_id -> channel_id/message_id` so events can be rehydrated across bot restarts. This is not required for the first Discord test version because active event state already persists in SQLite.

## Autocomplete details

### `/use item`

Flow:

1. Discord autocomplete event fires.
2. Convert author ID to `PlayerId`.
3. Run or directly use inventory store to list items.
4. Return up to 25 choices.

Choice name format:

```text
Lucky Dice — 3fa85f64
```

Choice value:

```text
3fa85f64-5717-4562-b3fc-2c963f66afa6
```

For clean layering, the core can later expose an `InventoryAutocompleteCommand`, but direct read through stores is acceptable in the Discord adapter if the `Game` exposes query helpers or if the command already returns structured inventory data.

### `/buy item`

Use static shop keys from the shop catalog. It may be worth exposing a public `ShopItemKind::keys()` / catalog helper from core.

### `/admin give item`

Use `AdminItemKind::keys()`.

## Implementation phases

### Phase 1 — Discord crate skeleton

- Add `mr-roller-discord` to workspace.
- Add dependencies: `poise`, `serenity`, `tokio`, `tracing`, `tracing-subscriber`, `mr-roller`.
- Extend `Settings` with `[discord]` config.
- Build `Arc<Game>` with SQLite and configured event settings.
- Start bot and register a simple `/ping` command.

Acceptance:

- Bot starts locally.
- `/ping` works in dev guild.

### Phase 2 — Basic player commands

Implement:

- `/start`
- `/inventory`
- `/shop`
- `/buy`
- `/leaderboard`

Use embeds for inventory/shop/leaderboard.

Acceptance:

- A Discord user can start, view inventory/shop, buy an item, and see leaderboard.

### Phase 3 — Item usage with autocomplete

Implement:

- `/use item:<autocomplete>`

Autocomplete lists current inventory items.

Acceptance:

- User can select an item by name from Discord autocomplete.
- Dice roll response is shown as an embed or polished message.

### Phase 4 — Admin commands

Implement admin command group:

- `/admin give`
- `/admin coins`
- `/admin set-admin`
- `/admin event spawn-random-item`

Acceptance:

- Admin commands respect the core admin checks.
- Admin can manually spawn a random item event in Discord.

### Phase 5 — Event buttons

Implement rendering and handlers for random item spawn events:

- Spawn event embed in configured channel.
- Add `Claim` and `Trash` buttons.
- Button handlers run core event commands.
- Edit message after claim/trash and remove buttons.

Acceptance:

- Event can be claimed by exactly one user.
- Event can be trashed by a user.
- Claimed/trashed event cannot be claimed again.

### Phase 6 — Background scheduler

Run `EventScheduler` in the Discord server process.

Acceptance:

- Bot runs 24/7 and checks for events on configured interval.
- Spawned events are posted automatically to `home_channel_id`.

### Phase 7 — Operational polish

- Add structured logs with `tracing`.
- Add graceful shutdown.
- Add clear startup validation for missing token/channel/database config.
- Add README instructions for Discord setup and permissions.
- Consider Dockerfile/systemd/fly.io/railway deployment later.

## Testing strategy

Unit tests remain mostly in `mr-roller`.

For Discord crate:

- Unit test render helpers where possible.
- Unit test custom ID parsing: `event:claim:<uuid>`.
- Keep Discord API integration mostly manual at first.
- Add a dev guild config for fast command iteration.

Manual smoke test checklist:

1. Start bot with SQLite config.
2. Run `/start`.
3. Run `/inventory`.
4. Admin grants coins.
5. Buy dice from `/shop`.
6. Use dice with autocomplete.
7. Admin spawns random item event.
8. Claim event with button.
9. Try claiming again from another account and verify it fails.
10. Let scheduler spawn an event automatically.

## Open decisions

- Whether event messages should be rehydrated and cleaned up after bot restart.
- Whether Discord command registration should be guild-only for now or support global registration immediately.
- Whether admin bootstrap IDs should use Discord user snowflakes only, or support separate configured admin roles later.

Recommendation for first Discord version: use guild commands, Discord user ID admins, and no event message rehydration. Add role-based admin and message rehydration once the first test version works.
