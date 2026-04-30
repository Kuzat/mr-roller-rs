# Public Discord Multi-Tenant Implementation Todo

Plan: `docs/plans/2026-04-29-public-discord-multitenant-plan.md`

## Progress

- [x] Create implementation branch `public-discord-multitenant`.
- [ ] Phase 1 — Update Discord config for service-level Postgres public mode.
- [ ] Phase 2 — Add Postgres multi-tenant storage and migrations in `mr-roller-discord`.
- [ ] Phase 3 — Add `DiscordGameRegistry` for setup and game resolution.
- [ ] Phase 4 — Implement `/setup channel:<text-channel>`.
- [ ] Phase 5 — Route gameplay/admin commands through the registry by guild/channel.
- [ ] Phase 6 — Replace single-channel event scheduler with multi-game scheduler.
- [ ] Phase 7 — Document install/deployment basics.
- [ ] Run formatting and tests.

## Notes

- The source plan is currently untracked in this working tree; keep it with the implementation branch.
