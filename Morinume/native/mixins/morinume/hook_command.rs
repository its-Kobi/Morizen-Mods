// HEAD of `native_command`: `/morinume`.
fn morinume_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    crate::game::morinume_hooks::command(game, command, ran);
}
