//! The mod's JSON config: profiles, per-module toggles and settings, HUD
//! anchors (capability `storage`, one key, `config`).
//!
//! Every field is `#[serde(default)]`, so an older or hand-edited file loads
//! with the missing parts defaulted instead of being thrown away.

use std::collections::BTreeMap;

use horizon_mod_sdk::storage;
use serde::{Deserialize, Serialize};

/// The one storage key; a profile is a section of it, not a key of its own.
pub const STORAGE_KEY: &str = "config";
/// Bumped when a migration is needed; unknown newer files load as-is.
pub const SCHEMA: u32 = 1;
/// The profile every config starts with.
pub const DEFAULT_PROFILE: &str = "Default";

/// A value the player picked for a setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Index(u32),
}

impl Value {
    pub fn as_bool(self) -> bool {
        matches!(self, Value::Bool(true))
    }
    pub fn as_index(self) -> u32 {
        match self {
            Value::Index(i) => i,
            Value::Bool(_) => 0,
        }
    }
}

/// What one profile remembers about one module.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleState {
    /// None until the player first toggles the card: the module's own default.
    pub enabled: Option<bool>,
    pub settings: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub modules: BTreeMap<String, ModuleState>,
}

/// A HUD module's panel position. `corner` is `top-left`, `top-right`,
/// `bottom-left` or `bottom-right`; `x`/`y` nudge it in GUI pixels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Anchor {
    pub corner: Corner,
    pub x: f32,
    pub y: f32,
}

impl Default for Anchor {
    fn default() -> Self {
        Anchor {
            corner: Corner::TopLeft,
            x: 4.0,
            y: 4.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    #[allow(dead_code)] // Tier 1's HUD modules read and write anchors.
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];

    #[allow(dead_code)] // Tier 1's anchor editor prints the key.
    pub fn key(self) -> &'static str {
        match self {
            Corner::TopLeft => "top-left",
            Corner::TopRight => "top-right",
            Corner::BottomLeft => "bottom-left",
            Corner::BottomRight => "bottom-right",
        }
    }

    /// The SDK's corner for a stored key.
    #[allow(dead_code)] // Tier 1's HUD panels call this.
    pub fn corner(self) -> horizon_mod_sdk::hud::Corner {
        use horizon_mod_sdk::hud::Corner as C;
        match self {
            Corner::TopLeft => C::TopLeft,
            Corner::TopRight => C::TopRight,
            Corner::BottomLeft => C::BottomLeft,
            Corner::BottomRight => C::BottomRight,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema: u32,
    pub active_profile: String,
    pub profiles: BTreeMap<String, Profile>,
    /// HUD panel positions, by module id; shared by every profile.
    pub hud: BTreeMap<String, Anchor>,
    /// The menu's key, as options.txt writes it.
    pub keybind: String,
    /// Grid (`true`) or list (`false`) on the MODS tab.
    pub grid: bool,
    /// Profiles are saved on every change; this only silences the log line.
    pub save_count: u32,
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Config {
            schema: SCHEMA,
            active_profile: DEFAULT_PROFILE.to_string(),
            profiles: BTreeMap::new(),
            hud: BTreeMap::new(),
            keybind: "key.keyboard.right.shift".to_string(),
            grid: true,
            save_count: 0,
        };
        config
            .profiles
            .insert(DEFAULT_PROFILE.to_string(), Profile::default());
        config
    }
}

impl Config {
    /// The stored config, or the default one when nothing is saved yet or the
    /// file is not ours. Never fails: a broken file must not keep the menu off.
    pub fn load() -> Config {
        let Some(bytes) = storage::get(STORAGE_KEY) else {
            return Config::default();
        };
        let Ok(text) = String::from_utf8(bytes) else {
            crate::log::warn("Morinume: config is not UTF-8; using defaults");
            return Config::default();
        };
        match serde_json::from_str::<Config>(&text) {
            Ok(mut config) => {
                config.schema = SCHEMA;
                config.ensure_profile();
                config
            }
            Err(e) => {
                crate::log::warn(&format!("Morinume: config unreadable ({e}); using defaults"));
                Config::default()
            }
        }
    }

    /// Writes the whole config as one JSON object. Called after every change:
    /// the file is a few hundred bytes, so the write is cheaper than a plan.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn save(&mut self) -> Result<(), String> {
        self.ensure_profile();
        self.save_count = self.save_count.wrapping_add(1);
        let text = serde_json::to_string(self).map_err(|e| e.to_string())?;
        storage::set(STORAGE_KEY, text.as_bytes())
    }

    fn ensure_profile(&mut self) {
        self.profiles.entry(self.active_profile.clone()).or_default();
    }

    pub fn profile(&self) -> Option<&Profile> {
        self.profiles.get(&self.active_profile)
    }

    fn profile_mut(&mut self) -> &mut Profile {
        let name = self.active_profile.clone();
        self.profiles.entry(name).or_default()
    }

    pub fn module(&self, id: &str) -> Option<&ModuleState> {
        self.profile().and_then(|p| p.modules.get(id))
    }

    /// None when the player never toggled it: the module's default applies.
    pub fn enabled(&self, id: &str) -> Option<bool> {
        self.module(id).and_then(|m| m.enabled)
    }

    pub fn set_enabled(&mut self, id: &str, on: bool) {
        self.profile_mut().modules.entry(id.to_string()).or_default().enabled = Some(on);
    }

    pub fn setting(&self, id: &str, key: &str) -> Option<Value> {
        self.module(id).and_then(|m| m.settings.get(key).copied())
    }

    pub fn set_setting(&mut self, id: &str, key: &str, value: Value) {
        self.profile_mut()
            .modules
            .entry(id.to_string())
            .or_default()
            .settings
            .insert(key.to_string(), value);
    }

    pub fn anchor(&self, id: &str) -> Anchor {
        self.hud.get(id).cloned().unwrap_or_default()
    }

    pub fn set_anchor(&mut self, id: &str, anchor: Anchor) {
        self.hud.insert(id.to_string(), anchor);
    }

    /// Profile names, `Default` first, the rest alphabetical.
    pub fn profile_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.profiles.keys().cloned().collect();
        names.sort_by_key(|n| (n != DEFAULT_PROFILE, n.clone()));
        names
    }

    /// Replaces the active profile's modules with a copy of the current ones
    /// and activates it. Returns the new name, or None at the 32nd profile.
    pub fn save_as_new(&mut self) -> Option<String> {
        if self.profiles.len() >= 32 {
            return None;
        }
        let copy = self.profile().cloned().unwrap_or_default();
        let mut n = 2;
        let name = loop {
            let name = format!("Profile {n}");
            if !self.profiles.contains_key(&name) {
                break name;
            }
            n += 1;
        };
        self.profiles.insert(name.clone(), copy);
        self.active_profile = name.clone();
        Some(name)
    }

    /// Switches profiles; false for an unknown name.
    pub fn activate(&mut self, name: &str) -> bool {
        if self.profiles.contains_key(name) {
            self.active_profile = name.to_string();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_usable_without_a_file() {
        let config = Config::default();
        assert_eq!(config.active_profile, DEFAULT_PROFILE);
        assert_eq!(config.keybind, "key.keyboard.right.shift");
        assert!(config.grid);
        assert_eq!(config.anchor("fps"), Anchor::default());
    }

    #[test]
    fn round_trips_through_json() {
        let mut config = Config::default();
        config.set_enabled("demo", true);
        config.set_setting("demo", "greet", Value::Bool(false));
        config.set_setting("fps", "unit", Value::Index(2));
        config.set_anchor(
            "fps",
            Anchor {
                corner: Corner::BottomRight,
                x: 2.0,
                y: 30.0,
            },
        );
        let text = serde_json::to_string(&config).unwrap();
        let back: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(config, back);
        assert_eq!(back.enabled("demo"), Some(true));
        assert_eq!(back.setting("fps", "unit"), Some(Value::Index(2)));
        assert_eq!(back.anchor("fps").corner, Corner::BottomRight);
    }

    #[test]
    fn profiles_start_as_a_copy_and_have_unique_names() {
        let mut config = Config::default();
        config.set_enabled("demo", true);
        let first = config.save_as_new().unwrap();
        assert_eq!(first, "Profile 2");
        assert_eq!(config.enabled("demo"), Some(true));

        config.set_enabled("demo", false);
        let second = config.save_as_new().unwrap();
        assert_eq!(second, "Profile 3");
        assert_eq!(config.profiles.len(), 3);

        assert!(config.activate(DEFAULT_PROFILE));
        assert_eq!(config.enabled("demo"), Some(true));
        assert!(!config.activate("Nope"));
    }

    #[test]
    fn unknown_fields_and_missing_sections_load() {
        let config: Config =
            serde_json::from_str(r#"{"schema":1,"keybind":"key.keyboard.m","future":1}"#).unwrap();
        assert_eq!(config.keybind, "key.keyboard.m");
        assert_eq!(config.active_profile, DEFAULT_PROFILE);
    }
}
