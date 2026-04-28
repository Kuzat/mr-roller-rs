# TODOs

## Documentation
- [x] Create a `README.md` in the repository root explaining the workspace layout and how to build each crate.
- [x] Document library usage for `mr-roller` crate including how to initialize a game and use items.
- [x] Document CLI usage once compilation issues are resolved.

## Library — Restructure (2026-04-28)
- [x] Phase 1: Async store traits with in-memory implementations
- [x] Phase 2: Command system, Response types, and Game dispatcher
- [x] Phase 3: `define_items!` macro and `GameItem` trait
- [x] Phase 4: New items — LuckyDice, CursedDice
- [x] Phase 5: CLI rewrite against new API
- [x] Cooldown system: one roll per day by default, configurable duration/reset, reroll token reset

## Library — Remaining
- [ ] Add `serde` derives to items for database serialization
- [ ] Phase 6: Database-backed stores (SQLite via sqlx)
- [ ] Implement Shop and Event game systems
- [ ] Implement partial item use / CompletedUseable workflow
- [ ] Consider extracting `Game`/`Context` into a builder pattern

## Testing & CI
- [ ] Add integration tests for CLI
- [ ] Set up continuous integration to run `cargo test` on each commit.

## Items — Ideas
- [ ] More dice: ElementalDice, BoostedDice, etc.
- [ ] More tokens: FreezeToken, StealToken
- [ ] Item rarity system
