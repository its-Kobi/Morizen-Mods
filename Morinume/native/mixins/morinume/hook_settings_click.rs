// HEAD of `native_settings_click`: a click on one of those buttons.
fn morinume_settings_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::morinume_hooks::settings_click(game, mod_id, button);
}
