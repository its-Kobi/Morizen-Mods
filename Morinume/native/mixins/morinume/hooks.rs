// Morinume's game glue (injected as `crate::game::morinume_hooks`).
//
// The pure logic lives in `morinume`: this module loads and saves its config
// as one JSON string, routes the hook points to it and paints the menu. The
// menu is a HUD overlay with keyboard navigation: native hook points carry no
// cursor position yet.

use std::sync::{Mutex, OnceLock};

use crate::game::morinume::{self, Config, Menu, Paint, Registry};

/// What survives between hook calls: the modules, their config and the menu.
struct State {
    registry: Registry,
    menu: Menu,
    loaded: bool,
    ticks: u64,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(State {
            registry: Registry::new(Config::default()),
            menu: Menu::default(),
            loaded: false,
            ticks: 0,
        })
    })
}

/// Locks the state; a panic elsewhere must not switch the menu off for good.
fn lock() -> std::sync::MutexGuard<'static, State> {
    state().lock().unwrap_or_else(|e| e.into_inner())
}

/// Reads the saved config the first time a hook runs for this game.
fn load(game: &crate::game::Game<'_>, st: &mut State) {
    if st.loaded {
        return;
    }
    st.loaded = true;
    let config = game
        .native_setting(morinume::ID, "config")
        .map(|text| Config::from_json(&text))
        .unwrap_or_default();
    st.registry = Registry::new(config);
}

fn save(game: &crate::game::Game<'_>, st: &State) {
    game.native_set_setting(morinume::ID, "config", &st.registry.config.to_json());
}

/// The HUD: the menu overlay while it is open.
pub(crate) fn hud(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) {
    if !game.native_on(morinume::ID) {
        return;
    }
    let mut guard = lock();
    let st = &mut *guard;
    load(game, st);
    if !st.menu.open {
        return;
    }
    let paints = st.menu.paint(&st.registry, batch.width, batch.height);
    for paint in paints {
        match paint {
            Paint::Rect { x, y, w, h, argb } => game.native_fill(batch, [x, y, w, h], argb),
            Paint::Text {
                text,
                x,
                y,
                rgb,
                shadow,
            } => game.native_text(batch, &text, [x, y], rgb, shadow),
        }
    }
}

/// Once a tick: the enabled modules tick.
pub(crate) fn tick(game: &mut crate::game::Game<'_>) {
    if !game.native_on(morinume::ID) {
        return;
    }
    let mut guard = lock();
    let st = &mut *guard;
    load(game, st);
    st.ticks = st.ticks.wrapping_add(1);
    st.registry.tick(st.ticks);
}

/// Keys: the menu key opens and closes the menu, the menu takes the rest.
pub(crate) fn key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_on(morinume::ID) {
        return;
    }
    let menu_key = game.native_mod_key(morinume::ID, "menu", morinume::DEFAULT_MENU_KEY);
    let mut guard = lock();
    let st = &mut *guard;
    load(game, st);
    if key == menu_key {
        st.menu.toggle();
        return;
    }
    if !st.menu.open {
        return;
    }
    if st.menu.key(&mut st.registry, key) {
        save(game, st);
    }
}

/// `/morinume ...`: every argument goes to the menu.
pub(crate) fn command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    let command = command.trim();
    let Some(rest) = command.strip_prefix("morinume") else {
        return;
    };
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return;
    }
    *ran = true;
    if !game.native_on(morinume::ID) {
        game.native_chat("Morinume is turned off in Options > Mods");
        return;
    }
    let mut guard = lock();
    let st = &mut *guard;
    load(game, st);
    match st.menu.command(&mut st.registry, rest.trim()) {
        Ok(Some(feedback)) => {
            save(game, st);
            game.native_chat(&feedback);
        }
        Ok(None) => {}
        Err(error) => game.native_chat(&format!("Morinume: {error}")),
    }
}

/// Options > Mods: Morinume's row, its key and its description.
pub(crate) fn mods(
    _game: &mut crate::game::Game<'_>,
    mods: &mut Vec<crate::game::native_hooks::NativeMod>,
) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: morinume::ID.to_string(),
        title: format!("Morinume {}", morinume::VERSION),
        description: "Morizen's mods menu: every mod, its options and its switch, in one place."
            .to_string(),
        keys: vec![[
            morinume::KEY_MENU.to_string(),
            "Morinume menu".to_string(),
            morinume::DEFAULT_MENU_KEY.to_string(),
        ]],
    });
}

/// The buttons of Morinume's settings page.
pub(crate) fn settings(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    if mod_id != morinume::ID {
        return;
    }
    let mut guard = lock();
    let st = &mut *guard;
    load(game, st);
    let demo = st
        .registry
        .modules
        .iter()
        .position(|m| m.id() == "demo")
        .map(|i| st.registry.enabled(i));
    let profile = st.registry.config.active_profile.clone();
    buttons.push(format!("Profile: {profile}"));
    buttons.push(match demo {
        Some(true) => "Demo Module: ON".to_string(),
        Some(false) => "Demo Module: OFF".to_string(),
        None => "Demo Module: missing".to_string(),
    });
}

/// A click on one of those buttons.
pub(crate) fn settings_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id != morinume::ID {
        return;
    }
    let mut guard = lock();
    let st = &mut *guard;
    load(game, st);
    if button == 1
        && let Some(index) = st.registry.modules.iter().position(|m| m.id() == "demo")
    {
        st.registry.toggle(index);
        save(game, st);
    }
}
