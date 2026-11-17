// HEAD of `native_hud`: the menu overlay.
fn morinume_hud(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) {
    crate::game::morinume_hooks::hud(game, batch);
}
