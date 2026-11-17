// HEAD of `native_mods`: Options > Mods' row for Morinume.
fn morinume_mods(
    game: &mut crate::game::Game<'_>,
    mods: &mut Vec<crate::game::native_hooks::NativeMod>,
) {
    crate::game::morinume_hooks::mods(game, mods);
}
