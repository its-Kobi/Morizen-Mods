// HEAD of `native_key`: the menu key and the menu's navigation.
fn morinume_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::morinume_hooks::key(game, key, pressed);
}
