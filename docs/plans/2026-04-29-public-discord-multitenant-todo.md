# Public Discord Multi-Tenant Implementation Todo

Plan: `docs/plans/2026-04-29-public-discord-multitenant-plan.md`

## Progress

- [x] Create implementation branch `public-discord-multitenant`.
- [x] Phase 1 — Update Discord config for service-level Postgres public mode.
- [x] Phase 2 — Add Postgres multi-tenant storage and migrations in `mr-roller-discord`.
- [x] Phase 3 — Add `DiscordGameRegistry` for setup and game resolution.
- [x] Phase 4 — Implement `/setup channel:<text-channel>`.
- [x] Phase 5 — Route gameplay/admin commands through the registry by guild/channel.
- [x] Phase 6 — Replace single-channel event scheduler with multi-game scheduler.
- [ ] Phase 7 — Document install/deployment basics.
- [x] Run formatting and compile check (`cargo fmt`, `cargo check -p mr-roller-discord`).

## Notes

- Public Discord runtime now requires a PostgreSQL `database.url`.
- Local/self-hosted SQLite support remains in the core crate and CLI; the Discord binary has moved to Postgres multi-tenancy.
