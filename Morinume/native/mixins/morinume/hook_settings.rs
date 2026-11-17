// HEAD of `native_settings`: the buttons of Morinume's settings page.
fn morinume_settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::morinume_hooks::settings(game, mod_id, buttons);
}
