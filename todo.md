# TODOs

## Documentation
- [x] Create a `README.md` in the repository root explaining the workspace layout and how to build each crate.
- [x] Document library usage for `mr-roller` crate including how to initialize a game and use items.
- [ ] Document CLI usage once compilation issues are resolved.

## Code Improvements
- Fix `mr-roller-cli` so it compiles; imports and method calls are outdated.
- Implement missing game commands listed as `NO` in `game.rs` (e.g., Inventory, Shop, Event, Help).
- Extend `MrRollerState` with database support instead of only in‑memory state.
- Revisit the commented out `CompletedUseable`/`UnCompletedUseable` logic in `item.rs` and design partial item use.

## Testing & CI
- Add unit and integration tests for the CLI and library.
- Set up continuous integration to run `cargo test` on each commit.

