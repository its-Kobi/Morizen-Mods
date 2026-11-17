//! Morinume: Morizen's mods menu.
//!
//! Tier 0: one menu screen over a registry of modules, one card component,
//! toggles and settings saved as JSON in the mod's `storage`, profiles, and a
//! demo module that proves the pipeline. Right Shift or `/morinume` opens it.
//!
//! The game's `ui` capability draws a list of the menus' own widgets, so the
//! cards are icon + name + OPTIONS + ENABLED/DISABLED rows; the painted dark
//! card grid of the design needs a game-side overlay screen (none in 0.1).

mod config;
mod log;
mod menu;
mod module;
mod modules;
mod registry;

use horizon_mod_sdk::{input, ui, Mod, Registration};

use crate::menu::MenuState;
use crate::registry::Registry;

struct Morinume {
    registry: Registry,
    menu: MenuState,
}

impl Mod for Morinume {
    fn init() -> Result<(Self, Registration), String> {
        let registry = Registry::new();
        let key = registry.config().keybind.clone();
        if let Err(e) = input::register_key("menu", "Open Morinume", &key) {
            crate::log::warn(&format!("Morinume: menu key {key}: {e}"));
        }
        let state = Morinume {
            registry,
            menu: MenuState::default(),
        };
        crate::log::info("Morinume loaded (Right Shift or /morinume)");
        let hooks = Registration::new()
            .command("morinume")
            .command("mods")
            .on_tick();
        Ok((state, hooks))
    }

    fn tick(&mut self, tick: u64) {
        for event in input::poll() {
            if let input::Event::KeyDown(name) = event {
                if name == "menu" {
                    self.menu.toggle(&self.registry);
                }
            }
        }
        self.registry.tick(tick);
    }

    fn ui_select(&mut self, id: u32, row: u32) {
        if self.menu.select(&mut self.registry, id, row) {
            self.menu.push(&self.registry);
        }
    }

    fn ui_closed(&mut self) {
        self.menu.open = false;
    }

    fn ui_settings_opened(&mut self) {
        // Options > Mods > Morinume shows the SETTINGS tab's rows.
        ui::set_settings(&self.menu.settings_widgets(&self.registry));
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        self.menu.command(&mut self.registry, args)
    }
}

horizon_mod_sdk::export_mod!(Morinume);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::modules;

    #[test]
    fn a_fresh_menu_shows_the_demo_card() {
        let registry = Registry::with_config(modules::all(), Config::default());
        let menu = MenuState::default();
        let screen = menu.screen(&registry);
        assert_eq!(screen.title, "MORINUME");
        assert!(screen
            .widgets
            .iter()
            .any(|w| matches!(w, ui::Widget::Item(i) if i.name.starts_with("Demo Module"))));
    }
}
