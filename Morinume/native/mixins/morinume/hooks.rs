// Morinume's game glue (injected as `crate::game::morinume_hooks`).
//
// The pure logic lives in `morinume`: this module loads and saves its config
// as one JSON string, gathers `HudInfo` through the native hook helpers, and
// paints the menu and the HUD panels. The menu is a HUD overlay with keyboard
// navigation: native hook points carry no cursor position yet.

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::game::morinume::{self, Config, HudInfo, Keys, Menu, Paint, Registry};

/// What survives between hook calls: the modules, their config, the menu and
/// the last menu paint (rebuilt only when something visible changed).
struct State {
    registry: Registry,
    menu: Menu,
    loaded: bool,
    ticks: u64,
    /// The menu or the config changed since the cached paint.
    dirty: bool,
    /// The cached menu paint and the screen it was laid out for.
    cache: Option<(f32, f32, Vec<Paint>)>,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(State {
            registry: Registry::new(Config::default()),
            menu: Menu::default(),
            loaded: false,
            ticks: 0,
            dirty: true,
            cache: None,
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
    st.dirty = true;
}

fn save(game: &crate::game::Game<'_>, st: &State) {
    game.native_set_setting(morinume::ID, "config", &st.registry.config.to_json());
}

/// Whether a vanilla mapping (`key.forward`, `key.attack`...) is held.
fn held(game: &mut crate::game::Game<'_>, name: &str) -> bool {
    game.native_mapping(name)
        .is_some_and(|(down, _)| down)
}

/// What the HUD modules may show this frame, from the game's own counters.
fn gather(game: &mut crate::game::Game<'_>, st: &State) -> HudInfo {
    let (fps, slowest, low) = game.native_frame_rate();
    let (attack, _use) = game.native_clicks();
    let keys = Keys {
        forward: held(game, "key.forward"),
        back: held(game, "key.back"),
        left: held(game, "key.left"),
        right: held(game, "key.right"),
        jump: held(game, "key.jump"),
        sneak: held(game, "key.sneak"),
        sprint: held(game, "key.sprint"),
        attack: held(game, "key.attack"),
        use_item: held(game, "key.use"),
    };
    let mut info = HudInfo {
        fps,
        slowest,
        low,
        playtime: st.ticks,
        cps: attack,
        keys,
        menu_open: st.menu.open,
        ..HudInfo::default()
    };
    if let Some(player) = game.native_player() {
        info.x = player.x;
        info.y = player.y;
        info.z = player.z;
        info.yaw = player.yaw;
        info.dimension = player.dimension.to_string();
    }
    info
}

/// A paint outside the `w` x `h` screen is skipped before it reaches the GPU.
fn on_screen(paint: &Paint, w: f32, h: f32) -> bool {
    let (x, y, pw, ph) = match paint {
        Paint::Rect { x, y, w, h, .. } => (*x, *y, *w, *h),
        Paint::Text { x, y, .. } => (*x, *y, 1.0, 1.0),
    };
    pw > 0.0 && ph > 0.0 && x < w && y < h && x + pw > 0.0 && y + ph > 0.0
}

fn draw(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    paints: &[Paint],
) -> u32 {
    let (w, h) = (batch.width, batch.height);
    let mut culled = 0;
    for paint in paints {
        if !on_screen(paint, w, h) {
            culled += 1;
            continue;
        }
        match paint {
            Paint::Rect { x, y, w, h, argb } => game.native_fill(batch, [*x, *y, *w, *h], *argb),
            Paint::Text {
                text,
                x,
                y,
                rgb,
                shadow,
            } => game.native_text(batch, text, [*x, *y], *rgb, *shadow),
        }
    }
    culled
}

/// The HUD: the menu while it is open, else the enabled modules' panels.
pub(crate) fn hud(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) {
    if !game.native_on(morinume::ID) || !game.native_playing() {
        return;
    }
    let mut guard = lock();
    let st = &mut *guard;
    load(game, st);
    let (w, h) = (batch.width, batch.height);
    if st.menu.open {
        let stale = st.dirty
            || st
                .cache
                .as_ref()
                .is_none_or(|(cw, ch, _)| *cw != w || *ch != h);
        if stale {
            let started = Instant::now();
            let paints = st.menu.paint(&st.registry, w, h);
            st.menu.paint_us = started.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;
            st.menu.paint_shapes = paints.len() as u32;
            st.cache = Some((w, h, paints));
            st.dirty = false;
        }
        let culled = match st.cache.as_ref() {
            Some((_, _, paints)) => draw(game, batch, paints),
            None => 0,
        };
        st.menu.paint_culled = culled;
        return;
    }
    st.cache = None;
    st.dirty = true;
    let info = gather(game, st);
    let paints = morinume::paint_hud(&st.registry, &info, w, h);
    st.menu.paint_culled = draw(game, batch, &paints);
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
        st.dirty = true;
        return;
    }
    if !st.menu.open {
        return;
    }
    if st.menu.key(&mut st.registry, key) {
        save(game, st);
        st.dirty = true;
    } else {
        st.dirty = true;
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
    let at = game
        .native_player()
        .map(|p| (p.x, p.y, p.z, p.dimension.to_string()));
    let mut guard = lock();
    let st = &mut *guard;
    load(game, st);
    st.dirty = true;
    match st.menu.command(&mut st.registry, rest.trim(), at) {
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

/// The buttons of Morinume's settings page: every module, then the profile.
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
    for i in 0..st.registry.modules.len() {
        buttons.push(format!(
            "{}: {}",
            st.registry.modules[i].name(),
            if st.registry.enabled(i) { "ON" } else { "OFF" }
        ));
    }
    buttons.push(format!(
        "Save profile as new ({})",
        st.registry.config.active_profile
    ));
}

/// A click on one of those buttons.
pub(crate) fn settings_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id != morinume::ID {
        return;
    }
    let mut guard = lock();
    let st = &mut *guard;
    load(game, st);
    st.dirty = true;
    if button < st.registry.modules.len() {
        st.registry.toggle(button);
    } else {
        if let Some(name) = st.registry.config.save_as_new() {
            game.native_chat(&format!("Morinume: saved {name}"));
        }
    }
    save(game, st);
}
