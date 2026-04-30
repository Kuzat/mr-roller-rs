# Discord Version TODO

Plan: `docs/plans/2026-04-29-discord-version-plan.md`

## Phase 1 — Discord crate skeleton

- [x] Add `mr-roller-discord` to workspace.
- [x] Add dependencies: `poise`, `serenity`, `tokio`, `tracing`, `tracing-subscriber`, `mr-roller`.
- [x] Extend `Settings` with `[discord]` config.
- [x] Build `Arc<Game>` with SQLite and configured event settings.
- [x] Start bot and register a simple `/ping` command.

## Phase 2 — Basic player commands

- [x] Implement `/start`.
- [x] Implement `/inventory`.
- [x] Implement `/shop`.
- [x] Implement `/buy`.
- [x] Implement `/leaderboard`.
- [x] Render inventory/shop/leaderboard with embeds.

## Phase 3 — Item usage with autocomplete

- [x] Implement `/use item:<autocomplete>`.
- [x] Autocomplete current player inventory items.
- [x] Render dice roll/use responses as polished Discord messages.

## Phase 4 — Admin commands

- [x] Implement `/admin give`.
- [x] Implement `/admin coins`.
- [x] Implement `/admin set-admin`.
- [x] Implement `/admin event spawn-random-item`.
- [x] Autocomplete admin item selection.

## Phase 5 — Event buttons

- [x] Render random item spawn embeds.
- [x] Add `Claim` and `Trash` buttons.
- [x] Handle event button interactions.
- [x] Edit event messages after claim/trash and remove buttons.

## Phase 6 — Background scheduler

- [x] Run `EventScheduler` in the Discord process.
- [x] Publish spawned events to `discord.home_channel_id`.

## Phase 7 — Operational polish

- [x] Add structured logs with `tracing`.
- [x] Add explicit graceful shutdown wiring.
- [x] Add startup validation for Discord token/channel/database config.
- [x] Add README instructions for Discord setup and permissions.

## Testing

- [ ] Unit test render helpers where possible.
- [x] Unit test event custom ID parsing.
- [x] Run `cargo fmt`.
- [x] Run `cargo test --workspace`.
- [x] Run `cargo check -p mr-roller-discord`.

## Follow-ups

- Manually smoke test in a dev Discord guild with a real bot token.
- Add message rehydration/cleanup after bot restarts if needed.
- Consider role-based admin permissions after the first test version works.
