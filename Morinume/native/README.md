# Morinume, native build

A **horizon-mixins** bundle: `horizon-mixin.toml` injects `morinume.rs` (pure
logic, also a standalone Cargo library) and `hooks.rs` (game glue) into the
game, and the files in `hooks/` are the typed adapters each injection compiles
inside its target function.

The game build is made by the launcher (`horizon-launcher native-build`) or by
this repository's GitHub Actions workflow; see `../agent.txt`.
