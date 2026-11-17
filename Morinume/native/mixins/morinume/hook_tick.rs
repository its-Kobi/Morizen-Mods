// HEAD of `native_tick`: tick the enabled modules.
fn morinume_tick(game: &mut crate::game::Game<'_>) {
    crate::game::morinume_hooks::tick(game);
}
