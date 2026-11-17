//! The module list plus the saved config: the only place that decides what is
//! on, calls the hooks and saves the file.

use crate::config::{Anchor, Config, Value};
use crate::module::{Module, Setting, SettingKind};
use crate::modules;

pub struct Registry {
    modules: Vec<Box<dyn Module>>,
    config: Config,
}

impl Registry {
    /// Loads the config, builds the catalogue and enables what it says.
    pub fn new() -> Self {
        Registry::with_config(modules::all(), Config::load())
    }

    pub(crate) fn with_config(modules: Vec<Box<dyn Module>>, config: Config) -> Self {
        let mut registry = Registry { modules, config };
        for i in 0..registry.modules.len() {
            if registry.is_enabled(i) {
                registry.modules[i].on_enable();
            }
        }
        registry
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn modules(&self) -> &[Box<dyn Module>] {
        &self.modules
    }

    pub fn module(&self, i: usize) -> &dyn Module {
        self.modules[i].as_ref()
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.modules.iter().position(|m| m.id() == id)
    }

    /// The saved override, else the module's own default.
    pub fn is_enabled(&self, i: usize) -> bool {
        let module = &self.modules[i];
        self.config.enabled(module.id()).unwrap_or_else(|| module.enabled_by_default())
    }

    /// Flips one module, calls its hook and saves. Returns the new state.
    pub fn toggle(&mut self, i: usize) -> bool {
        let on = !self.is_enabled(i);
        self.set_enabled(i, on);
        on
    }

    pub fn set_enabled(&mut self, i: usize, on: bool) {
        if self.is_enabled(i) == on {
            return;
        }
        let id = self.modules[i].id();
        self.config.set_enabled(id, on);
        if on {
            self.modules[i].on_enable();
        } else {
            self.modules[i].on_disable();
        }
        self.save();
    }

    /// The setting's stored value, else its declared default.
    pub fn setting(&self, i: usize, setting: &Setting) -> Value {
        let module = &self.modules[i];
        if let Some(value) = self.config.setting(module.id(), setting.key) {
            return value;
        }
        match setting.kind {
            SettingKind::Toggle => Value::Bool(setting.default_on),
            SettingKind::Choice(_) => Value::Index(0),
        }
    }

    pub fn setting_label(&self, i: usize, setting: &Setting) -> String {
        match setting.kind {
            SettingKind::Toggle => {
                let on = self.setting(i, setting).as_bool();
                format!("{}: {}", setting.label, if on { "ON" } else { "OFF" })
            }
            SettingKind::Choice(options) => {
                let row = self.setting(i, setting).as_index() as usize;
                let pick = options.get(row).copied().unwrap_or("?");
                format!("{}: {}", setting.label, pick)
            }
        }
    }

    /// A toggle flips, a choice moves to the next option. Saves.
    pub fn cycle_setting(&mut self, i: usize, setting: &Setting) {
        let value = match setting.kind {
            SettingKind::Toggle => Value::Bool(!self.setting(i, setting).as_bool()),
            SettingKind::Choice(options) => {
                let next = (self.setting(i, setting).as_index() + 1) % options.len().max(1) as u32;
                Value::Index(next)
            }
        };
        let id = self.modules[i].id();
        self.config.set_setting(id, setting.key, value);
        self.save();
    }

    /// Switches profile and brings every module's hook in line with it.
    pub fn activate_profile(&mut self, name: &str) -> bool {
        if !self.config.profiles.contains_key(name) {
            return false;
        }
        if self.config.active_profile == name {
            return true;
        }
        let before = self.enabled_snapshot();
        if !self.config.activate(name) {
            return false;
        }
        self.apply(before);
        self.save();
        true
    }

    /// Saves the current profile as a new one and switches to it.
    pub fn save_as_new_profile(&mut self) -> Option<String> {
        let name = self.config.save_as_new()?;
        self.save();
        Some(name)
    }

    fn enabled_snapshot(&self) -> Vec<bool> {
        (0..self.modules.len()).map(|i| self.is_enabled(i)).collect()
    }

    /// Calls on_enable/on_disable for every module whose state differs from
    /// the snapshot taken before the config changed.
    fn apply(&mut self, before: Vec<bool>) {
        for i in 0..self.modules.len() {
            let now = self.is_enabled(i);
            if before.get(i).copied().unwrap_or(false) != now {
                if now {
                    self.modules[i].on_enable();
                } else {
                    self.modules[i].on_disable();
                }
            }
        }
    }

    pub fn tick(&mut self, tick: u64) {
        for i in 0..self.modules.len() {
            if self.is_enabled(i) {
                self.modules[i].on_tick(tick);
            }
        }
    }

    #[allow(dead_code)] // Called from the frame hook once a HUD module needs it.
    pub fn render(&mut self, seconds: f32) {
        for i in 0..self.modules.len() {
            if self.is_enabled(i) {
                self.modules[i].on_render(seconds);
            }
        }
    }

    #[allow(dead_code)] // Tier 1's HUD modules place their panels with these.
    pub fn anchor(&self, id: &str) -> Anchor {
        self.config.anchor(id)
    }

    #[allow(dead_code)]
    pub fn set_anchor(&mut self, id: &str, anchor: Anchor) {
        self.config.set_anchor(id, anchor);
        self.save();
    }

    /// Writes the config; a failure is a log line, never a panic. Host builds
    /// (`cargo test`, clippy) have no storage, so they keep it in memory.
    fn save(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if let Err(e) = self.config.save() {
            crate::log::warn(&format!("Morinume: could not save the config: {e}"));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = &mut self.config;
    }
}

impl Default for Registry {
    fn default() -> Self {
        Registry::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry::with_config(modules::all(), Config::default())
    }

    #[test]
    fn the_dummy_module_is_the_first_card() {
        let registry = registry();
        assert_eq!(registry.module(0).id(), "demo");
        assert_eq!(registry.module(0).name(), "Demo Module");
        assert!(registry.is_enabled(0));
    }

    #[test]
    fn toggling_changes_the_state_and_the_saved_override() {
        let mut registry = registry();
        assert!(!registry.toggle(0));
        assert_eq!(registry.config().enabled("demo"), Some(false));
        assert!(registry.toggle(0));
        assert_eq!(registry.config().enabled("demo"), Some(true));
    }

    #[test]
    fn a_toggle_setting_starts_at_its_declared_default() {
        let registry = registry();
        let setting = &registry.module(0).settings()[0];
        assert!(registry.setting(0, setting).as_bool());
        assert_eq!(registry.setting_label(0, setting), "Log when enabled: ON");
    }
}
