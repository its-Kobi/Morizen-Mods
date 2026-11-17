# Morinume, native build

A **horizon-mixins** bundle. `mixins/morinume/horizon-mixin.toml` injects two
support modules into `crates/horizon-client/src/game.rs`: `morinume.rs` (pure
logic, also this repository's standalone Cargo library) and `hooks.rs` (game
glue: config files, HUD painting). The `hook_*.rs` files are the typed adapters
each HEAD injection compiles inside its target function.

Tier 0 is a keyboard-driven HUD overlay: Right Shift opens it, arrows move,
Enter toggles a card, O opens its options, Tab changes tab, Esc closes. The
config is saved as one JSON file in the mod's data folder.

```sh
cargo test        # the logic on the host, no game needed
cargo clippy --all-targets -- -D warnings
```

The game build is made by the launcher (`horizon-launcher native-build`) or by
this repository's GitHub Actions workflow (`native.yml`), never on a
contributor's machine. Pinned revisions are in `compatibility.json`.
