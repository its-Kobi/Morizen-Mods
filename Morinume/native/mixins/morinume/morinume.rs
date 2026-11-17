// Morinume's menu: config, modules, registry, menu state and the paint list.
//
// This file is both a standalone Cargo library (`cargo test` here) and the
// module `crate::game::morinume` injected into the game by
// `horizon-mixin.toml`. It touches no game type on purpose: the game glue in
// `hooks.rs` loads/saves the config, gathers `HudInfo` and turns `Paint`s into
// HUD shapes. horizon-mixins does not accept inner file attributes, so this
// header uses ordinary comments.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The mod's id: its Options > Mods row and its settings folder.
pub const ID: &str = "morinume";
pub const VERSION: &str = "0.2.0";
/// The key name on the mod's settings page.
pub const KEY_MENU: &str = "menu";
/// The menu's key until the player binds another one.
pub const DEFAULT_MENU_KEY: &str = "key.keyboard.right.shift";
pub const DEFAULT_PROFILE: &str = "Default";
/// At most this many profiles and waypoints.
pub const MAX_PROFILES: usize = 32;
pub const MAX_WAYPOINTS: usize = 128;

// ---------------------------------------------------------------------------
// Config (saved as one JSON string in `native_setting(game, "morinume", "config")`)
// ---------------------------------------------------------------------------

/// A HUD module's panel position; `corner` is `top-left`, `top-right`,
/// `bottom-left` or `bottom-right`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Anchor {
    pub corner: String,
    pub x: f32,
    pub y: f32,
}

impl Default for Anchor {
    fn default() -> Self {
        Anchor {
            corner: "top-left".to_string(),
            x: 4.0,
            y: 4.0,
        }
    }
}

impl Anchor {
    pub const CORNERS: [&'static str; 4] = ["top-left", "top-right", "bottom-left", "bottom-right"];

    /// `(x, y)` of a `w` x `h` panel on a `screen_w` x `screen_h` screen.
    pub fn at(&self, w: f32, h: f32, screen_w: f32, screen_h: f32) -> (f32, f32) {
        let x = if self.corner.ends_with("right") {
            screen_w - w - self.x
        } else {
            self.x
        };
        let y = if self.corner.starts_with("bottom") {
            screen_h - h - self.y
        } else {
            self.y
        };
        (x.max(0.0), y.max(0.0))
    }

    /// Moves the panel by `(dx, dy)`, keeping a margin on every edge.
    pub fn nudge(&mut self, dx: f32, dy: f32, screen: (f32, f32)) {
        self.x = (self.x + dx).clamp(0.0, screen.0.max(0.0));
        self.y = (self.y + dy).clamp(0.0, screen.1.max(0.0));
    }

    pub fn next_corner(&mut self) {
        let at = Anchor::CORNERS
            .iter()
            .position(|c| *c == self.corner)
            .unwrap_or(0);
        self.corner = Anchor::CORNERS[(at + 1) % Anchor::CORNERS.len()].to_string();
    }
}

/// A saved position the Waypoints tab lists and the tracker can follow.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Waypoint {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// `minecraft:overworld`, `minecraft:the_nether`...
    pub dim: String,
}

impl Default for Waypoint {
    fn default() -> Self {
        Waypoint {
            name: String::new(),
            x: 0.0,
            y: 64.0,
            z: 0.0,
            dim: "minecraft:overworld".to_string(),
        }
    }
}

impl Waypoint {
    /// Flat distance from `(x, z)`, so a Nether portal does not read wrong.
    pub fn distance(&self, x: f64, z: f64, dim: &str) -> Option<f64> {
        (self.dim == dim).then(|| ((self.x - x).powi(2) + (self.z - z).powi(2)).sqrt())
    }
}

/// What the player changed. Everything is `#[serde(default)]`, so an older or
/// hand-edited file keeps loading.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    /// Per module, in the active profile: no entry = the module's own default.
    pub enabled: BTreeMap<String, bool>,
    /// Per module, per setting key: `on`/`off` or a choice's index as text.
    pub settings: BTreeMap<String, BTreeMap<String, String>>,
    /// Saved profiles, each a copy of `enabled`.
    pub profiles: BTreeMap<String, BTreeMap<String, bool>>,
    pub active_profile: String,
    /// HUD panel positions, by module id; shared by every profile.
    pub hud: BTreeMap<String, Anchor>,
    pub waypoints: Vec<Waypoint>,
    /// The waypoint the tracker follows, by name.
    pub tracked: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Config {
            version: 2,
            enabled: BTreeMap::new(),
            settings: BTreeMap::new(),
            profiles: BTreeMap::new(),
            active_profile: DEFAULT_PROFILE.to_string(),
            hud: BTreeMap::new(),
            waypoints: Vec::new(),
            tracked: None,
        };
        config
            .profiles
            .insert(DEFAULT_PROFILE.to_string(), BTreeMap::new());
        config
    }
}

impl Config {
    pub fn from_json(text: &str) -> Config {
        let mut config = serde_json::from_str::<Config>(text).unwrap_or_default();
        config.normalise();
        config
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn normalise(&mut self) {
        if self.active_profile.is_empty() {
            self.active_profile = DEFAULT_PROFILE.to_string();
        }
        let enabled = self.enabled.clone();
        let profile = self
            .profiles
            .entry(self.active_profile.clone())
            .or_default();
        // Configs from before profiles kept their toggles in `enabled`.
        if profile.is_empty() && !enabled.is_empty() {
            *profile = enabled;
        }
        if self.tracked.as_ref().is_some_and(|name| {
            !self.waypoints.iter().any(|w| &w.name == name)
        }) {
            self.tracked = None;
        }
    }

    pub fn is_enabled(&self, id: &str, default: bool) -> bool {
        self.profiles
            .get(&self.active_profile)
            .and_then(|p| p.get(id).copied())
            .unwrap_or(default)
    }

    pub fn set_enabled(&mut self, id: &str, on: bool) {
        self.enabled.insert(id.to_string(), on);
        self.profiles
            .entry(self.active_profile.clone())
            .or_default()
            .insert(id.to_string(), on);
    }

    pub fn setting<'a>(&'a self, id: &str, key: &str) -> Option<&'a str> {
        self.settings
            .get(id)
            .and_then(|s| s.get(key))
            .map(String::as_str)
    }

    pub fn set_setting(&mut self, id: &str, key: &str, value: &str) {
        self.settings
            .entry(id.to_string())
            .or_default()
            .insert(key.to_string(), value.to_string());
    }

    /// Profile names, `Default` first, then alphabetical.
    pub fn profile_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.profiles.keys().cloned().collect();
        names.sort_by_key(|n| (n != DEFAULT_PROFILE, n.clone()));
        names
    }

    /// Saves the active profile's toggles as a new one and switches to it.
    pub fn save_as_new(&mut self) -> Option<String> {
        if self.profiles.len() >= MAX_PROFILES {
            return None;
        }
        let copy = self.profiles.get(&self.active_profile).cloned().unwrap_or_default();
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

    /// Creates a profile named `name` from the current toggles.
    pub fn create_profile(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || self.profiles.len() >= MAX_PROFILES || self.profiles.contains_key(name)
        {
            return false;
        }
        let copy = self.profiles.get(&self.active_profile).cloned().unwrap_or_default();
        self.profiles.insert(name.to_string(), copy);
        self.active_profile = name.to_string();
        true
    }

    pub fn delete_profile(&mut self, name: &str) -> bool {
        if name == DEFAULT_PROFILE || !self.profiles.contains_key(name) {
            return false;
        }
        self.profiles.remove(name);
        if self.active_profile == name {
            self.active_profile = DEFAULT_PROFILE.to_string();
        }
        true
    }

    pub fn activate(&mut self, name: &str) -> bool {
        if self.profiles.contains_key(name) {
            self.active_profile = name.to_string();
            true
        } else {
            false
        }
    }

    pub fn next_profile(&mut self, step: isize) -> bool {
        let names = self.profile_names();
        let Some(at) = names.iter().position(|n| *n == self.active_profile) else {
            return false;
        };
        let len = names.len() as isize;
        let next = ((at as isize + step) % len + len) % len;
        self.active_profile = names[next as usize].clone();
        true
    }

    pub fn anchor(&self, id: &str) -> Anchor {
        self.hud.get(id).cloned().unwrap_or_default()
    }

    pub fn anchor_mut(&mut self, id: &str) -> &mut Anchor {
        self.hud.entry(id.to_string()).or_default()
    }

    pub fn waypoint(&self, name: &str) -> Option<&Waypoint> {
        self.waypoints.iter().find(|w| w.name == name)
    }

    /// Adds (or moves) a waypoint; returns false at the cap or for a bad name.
    pub fn add_waypoint(&mut self, waypoint: Waypoint) -> bool {
        let name = waypoint.name.trim().to_string();
        if name.is_empty() {
            return false;
        }
        if let Some(old) = self.waypoints.iter_mut().find(|w| w.name == name) {
            *old = Waypoint { name, ..waypoint };
            return true;
        }
        if self.waypoints.len() >= MAX_WAYPOINTS {
            return false;
        }
        self.waypoints.push(Waypoint { name, ..waypoint });
        self.waypoints.sort_by(|a, b| a.name.cmp(&b.name));
        true
    }

    pub fn delete_waypoint(&mut self, name: &str) -> bool {
        let before = self.waypoints.len();
        self.waypoints.retain(|w| w.name != name);
        if self.tracked.as_deref() == Some(name) {
            self.tracked = None;
        }
        self.waypoints.len() != before
    }

    pub fn track(&mut self, name: Option<&str>) -> bool {
        match name {
            None | Some("none") => {
                self.tracked = None;
                true
            }
            Some(name) if self.waypoint(name).is_some() => {
                self.tracked = Some(name.to_string());
                true
            }
            Some(_) => false,
        }
    }

    pub fn tracked_waypoint(&self) -> Option<&Waypoint> {
        self.tracked.as_deref().and_then(|name| self.waypoint(name))
    }
}

// ---------------------------------------------------------------------------
// What the HUD reads
// ---------------------------------------------------------------------------

/// The game state a HUD module may show, gathered by `hooks.rs` once a frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HudInfo {
    pub fps: u32,
    pub slowest: u32,
    pub low: u32,
    /// In ticks, at 20 a second.
    pub playtime: u64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub dimension: String,
    pub cps: u32,
    pub keys: Keys,
    pub menu_open: bool,
}

/// Held keys, by vanilla mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Keys {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
    pub attack: bool,
    pub use_item: bool,
}

impl HudInfo {
    pub fn facing(&self) -> &'static str {
        // Vanilla: yaw 0 is south, 90 west, 180 north, 270 east.
        let yaw = self.yaw.rem_euclid(360.0);
        match ((yaw + 45.0) / 90.0) as u32 % 4 {
            0 => "south",
            1 => "west",
            2 => "north",
            _ => "east",
        }
    }

    pub fn dimension_name(&self) -> &str {
        self.dimension.rsplit(':').next().unwrap_or(&self.dimension)
    }

    pub fn clock(&self) -> String {
        let seconds = self.playtime / 20;
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            (seconds / 60) % 60,
            seconds % 60
        )
    }
}

/// A key on a keystrokes HUD: its label and whether it is held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    pub label: &'static str,
    pub pressed: bool,
}

// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------

/// The chip a card answers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Hud,
    Server,
    Mechanic,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Hud => "HUD",
            Category::Server => "SERVER",
            Category::Mechanic => "MECHANIC",
        }
    }
}

/// What a setting row does when activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    Toggle,
    /// One of `options`, kept as an index.
    Choice(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: SettingKind,
    pub default_on: bool,
}

impl Setting {
    pub const fn toggle(key: &'static str, label: &'static str, on: bool) -> Self {
        Setting {
            key,
            label,
            kind: SettingKind::Toggle,
            default_on: on,
        }
    }

    pub const fn choice(
        key: &'static str,
        label: &'static str,
        options: &'static [&'static str],
    ) -> Self {
        Setting {
            key,
            label,
            kind: SettingKind::Choice(options),
            default_on: false,
        }
    }
}

/// One card and its hooks. The menu only reads this.
pub trait Module: Send {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    /// Drawn as the card's icon: a small coloured square for now.
    fn accent(&self) -> u32 {
        0x5B9BD5
    }
    fn category(&self) -> Category;
    fn is_new(&self) -> bool {
        false
    }
    fn enabled_by_default(&self) -> bool {
        false
    }
    fn settings(&self) -> &'static [Setting] {
        &[]
    }
    fn on_enable(&mut self) {}
    fn on_disable(&mut self) {}
    fn on_tick(&mut self, _tick: u64) {}
    /// A short line the options page shows under the module, or nothing.
    fn status(&self) -> Option<String> {
        None
    }
    /// The lines this module adds to the HUD, empty for a menu-only module.
    fn hud(&self, _info: &HudInfo, _config: &Config) -> Vec<String> {
        Vec::new()
    }
    /// The keys a keystrokes-like module shows, drawn as chips.
    fn chips(&self, _info: &HudInfo, _config: &Config) -> Vec<Chip> {
        Vec::new()
    }
}

fn on(config: &Config, id: &str, setting: &Setting) -> bool {
    config
        .setting(id, setting.key)
        .map(|v| v == "on")
        .unwrap_or(setting.default_on)
}

const FPS_SETTINGS: &[Setting] = &[
    Setting::toggle("slowest", "Show slowest frame", true),
    Setting::toggle("low", "Show 1% low", true),
];

pub struct Fps;

impl Module for Fps {
    fn id(&self) -> &'static str {
        "fps"
    }
    fn name(&self) -> &'static str {
        "FPS Counter"
    }
    fn accent(&self) -> u32 {
        0x3FBF6F
    }
    fn category(&self) -> Category {
        Category::Hud
    }
    fn is_new(&self) -> bool {
        true
    }
    fn enabled_by_default(&self) -> bool {
        true
    }
    fn settings(&self) -> &'static [Setting] {
        FPS_SETTINGS
    }
    fn hud(&self, info: &HudInfo, config: &Config) -> Vec<String> {
        let mut lines = vec![format!("{} FPS", info.fps)];
        if on(config, self.id(), &FPS_SETTINGS[0]) {
            lines.push(format!("min {}", info.slowest));
        }
        if on(config, self.id(), &FPS_SETTINGS[1]) {
            lines.push(format!("1% {}", info.low));
        }
        lines
    }
    fn status(&self) -> Option<String> {
        Some("reads the game's own frame counter".to_string())
    }
}

pub struct Cps;

impl Module for Cps {
    fn id(&self) -> &'static str {
        "cps"
    }
    fn name(&self) -> &'static str {
        "CPS"
    }
    fn accent(&self) -> u32 {
        0xE0B45A
    }
    fn category(&self) -> Category {
        Category::Hud
    }
    fn enabled_by_default(&self) -> bool {
        true
    }
    fn hud(&self, info: &HudInfo, _config: &Config) -> Vec<String> {
        vec![format!("{} CPS", info.cps)]
    }
}

pub struct Keystrokes;

impl Module for Keystrokes {
    fn id(&self) -> &'static str {
        "keystrokes"
    }
    fn name(&self) -> &'static str {
        "Keystrokes"
    }
    fn accent(&self) -> u32 {
        0x5B9BD5
    }
    fn category(&self) -> Category {
        Category::Hud
    }
    fn is_new(&self) -> bool {
        true
    }
    fn enabled_by_default(&self) -> bool {
        true
    }
    fn settings(&self) -> &'static [Setting] {
        const SETTINGS: &[Setting] = &[
            Setting::toggle("sprint", "Show sprint", true),
            Setting::toggle("mouse", "Show mouse buttons", true),
            Setting::toggle("jump", "Show jump", true),
        ];
        SETTINGS
    }
    fn chips(&self, info: &HudInfo, config: &Config) -> Vec<Chip> {
        let k = info.keys;
        let mut chips = vec![
            Chip {
                label: "W",
                pressed: k.forward,
            },
            Chip {
                label: "A",
                pressed: k.left,
            },
            Chip {
                label: "S",
                pressed: k.back,
            },
            Chip {
                label: "D",
                pressed: k.right,
            },
        ];
        if on(config, self.id(), &self.settings()[2]) {
            chips.push(Chip {
                label: "SPC",
                pressed: k.jump,
            });
        }
        if on(config, self.id(), &self.settings()[0]) {
            chips.push(Chip {
                label: "SPR",
                pressed: k.sprint,
            });
        }
        if on(config, self.id(), &self.settings()[1]) {
            chips.push(Chip {
                label: "LMB",
                pressed: k.attack,
            });
            chips.push(Chip {
                label: "RMB",
                pressed: k.use_item,
            });
        }
        chips
    }
}

pub struct Coordinates;

impl Module for Coordinates {
    fn id(&self) -> &'static str {
        "coords"
    }
    fn name(&self) -> &'static str {
        "Coordinates"
    }
    fn accent(&self) -> u32 {
        0xCF5A5A
    }
    fn category(&self) -> Category {
        Category::Hud
    }
    fn enabled_by_default(&self) -> bool {
        true
    }
    fn settings(&self) -> &'static [Setting] {
        const SETTINGS: &[Setting] = &[
            Setting::toggle("chunk", "Show chunk", false),
            Setting::toggle("dimension", "Show dimension", false),
            Setting::toggle("facing", "Show facing", true),
        ];
        SETTINGS
    }
    fn hud(&self, info: &HudInfo, config: &Config) -> Vec<String> {
        let settings = self.settings();
        let mut lines = vec![format!(
            "XYZ {} {} {}",
            info.x.floor() as i64,
            info.y.floor() as i64,
            info.z.floor() as i64
        )];
        if on(config, self.id(), &settings[2]) {
            lines.push(format!("facing {}", info.facing()));
        }
        if on(config, self.id(), &settings[0]) {
            lines.push(format!(
                "chunk {} {}",
                (info.x.floor() / 16.0) as i64,
                (info.z.floor() / 16.0) as i64
            ));
        }
        if on(config, self.id(), &settings[1]) {
            lines.push(info.dimension_name().to_string());
        }
        lines
    }
}

pub struct Playtime;

impl Module for Playtime {
    fn id(&self) -> &'static str {
        "playtime"
    }
    fn name(&self) -> &'static str {
        "Playtime"
    }
    fn accent(&self) -> u32 {
        0x9B7BD5
    }
    fn category(&self) -> Category {
        Category::Hud
    }
    fn hud(&self, info: &HudInfo, _config: &Config) -> Vec<String> {
        vec![info.clock()]
    }
}

pub struct Tracker;

impl Module for Tracker {
    fn id(&self) -> &'static str {
        "tracker"
    }
    fn name(&self) -> &'static str {
        "Waypoint Tracker"
    }
    fn accent(&self) -> u32 {
        0xD5A05B
    }
    fn category(&self) -> Category {
        Category::Hud
    }
    fn hud(&self, info: &HudInfo, config: &Config) -> Vec<String> {
        let Some(waypoint) = config.tracked_waypoint() else {
            return vec!["no waypoint".to_string()];
        };
        match waypoint.distance(info.x, info.z, &info.dimension) {
            Some(distance) => vec![waypoint.name.clone(), format!("{distance:.0} m")],
            None => vec![waypoint.name.clone(), "another dimension".to_string()],
        }
    }
}

pub struct Performance;

impl Module for Performance {
    fn id(&self) -> &'static str {
        "performance"
    }
    fn name(&self) -> &'static str {
        "Performance"
    }
    fn accent(&self) -> u32 {
        0x4FC3C8
    }
    fn category(&self) -> Category {
        Category::Mechanic
    }
    fn is_new(&self) -> bool {
        true
    }
    fn enabled_by_default(&self) -> bool {
        true
    }
    fn settings(&self) -> &'static [Setting] {
        const SETTINGS: &[Setting] = &[
            Setting::toggle("cost", "Show menu paint cost", true),
            Setting::toggle("modules", "Show enabled modules", true),
        ];
        SETTINGS
    }
    fn status(&self) -> Option<String> {
        Some("the Performance tab's switches".to_string())
    }
}

/// Every module, in card order. One line per mod; the menu never changes.
pub fn modules() -> Vec<Box<dyn Module>> {
    vec![
        Box::new(Fps),
        Box::new(Cps),
        Box::new(Keystrokes),
        Box::new(Coordinates),
        Box::new(Playtime),
        Box::new(Tracker),
        Box::new(Performance),
    ]
}

/// The game's modules as a list, plus the config, and what a change needs.
pub struct Registry {
    pub modules: Vec<Box<dyn Module>>,
    pub config: Config,
}

impl Registry {
    pub fn new(config: Config) -> Self {
        let mut registry = Registry {
            modules: modules(),
            config,
        };
        registry.sync();
        registry
    }

    fn sync(&mut self) {
        for i in 0..self.modules.len() {
            if self.enabled(i) {
                self.modules[i].on_enable();
            } else {
                self.modules[i].on_disable();
            }
        }
    }

    pub fn enabled(&self, i: usize) -> bool {
        let module = &self.modules[i];
        self.config.is_enabled(module.id(), module.enabled_by_default())
    }

    pub fn toggle(&mut self, i: usize) -> bool {
        let on = !self.enabled(i);
        self.config.set_enabled(self.modules[i].id(), on);
        if on {
            self.modules[i].on_enable();
        } else {
            self.modules[i].on_disable();
        }
        on
    }

    pub fn enabled_count(&self) -> (usize, usize) {
        let enabled = (0..self.modules.len()).filter(|i| self.enabled(*i)).count();
        (enabled, self.modules.len())
    }

    /// A toggle's value; a choice's index is stored as text.
    pub fn setting_on(&self, i: usize, setting: &Setting) -> bool {
        self.config
            .setting(self.modules[i].id(), setting.key)
            .map(|v| v == "on")
            .unwrap_or(setting.default_on)
    }

    pub fn setting_label(&self, i: usize, setting: &Setting) -> String {
        match setting.kind {
            SettingKind::Toggle => format!(
                "{}: {}",
                setting.label,
                if self.setting_on(i, setting) {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            SettingKind::Choice(options) => {
                let row = self
                    .config
                    .setting(self.modules[i].id(), setting.key)
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);
                format!(
                    "{}: {}",
                    setting.label,
                    options.get(row).copied().unwrap_or("?")
                )
            }
        }
    }

    pub fn cycle_setting(&mut self, i: usize, setting: &Setting) {
        let value = match setting.kind {
            SettingKind::Toggle => {
                if self.setting_on(i, setting) {
                    "off".to_string()
                } else {
                    "on".to_string()
                }
            }
            SettingKind::Choice(options) => {
                let row = self
                    .config
                    .setting(self.modules[i].id(), setting.key)
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);
                ((row + 1) % options.len().max(1)).to_string()
            }
        };
        let id = self.modules[i].id();
        self.config.set_setting(id, setting.key, &value);
    }

    pub fn tick(&mut self, tick: u64) {
        for i in 0..self.modules.len() {
            if self.enabled(i) {
                self.modules[i].on_tick(tick);
            }
        }
    }

    pub fn huds(&self) -> Vec<usize> {
        self.modules
            .iter()
            .enumerate()
            .filter(|(i, m)| m.category() == Category::Hud && self.enabled(*i))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn activate_profile(&mut self, name: &str) -> bool {
        if !self.config.activate(name) {
            return false;
        }
        self.sync();
        true
    }

    pub fn next_profile(&mut self, step: isize) -> bool {
        if !self.config.next_profile(step) {
            return false;
        }
        self.sync();
        true
    }
}

// ---------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Mods,
    Settings,
    Waypoints,
    Performance,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Mods, Tab::Settings, Tab::Waypoints, Tab::Performance];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Mods => "MODS",
            Tab::Settings => "SETTINGS",
            Tab::Waypoints => "WAYPOINTS",
            Tab::Performance => "PERFORMANCE",
        }
    }

    fn next(self) -> Tab {
        match self {
            Tab::Mods => Tab::Settings,
            Tab::Settings => Tab::Waypoints,
            Tab::Waypoints => Tab::Performance,
            Tab::Performance => Tab::Mods,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    New,
    Hud,
    Server,
    Mechanic,
}

impl Filter {
    pub const ALL: [Filter; 5] = [
        Filter::All,
        Filter::New,
        Filter::Hud,
        Filter::Server,
        Filter::Mechanic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "ALL",
            Filter::New => "NEW",
            Filter::Hud => "HUD",
            Filter::Server => "SERVER",
            Filter::Mechanic => "MECHANIC",
        }
    }

    fn matches(self, module: &dyn Module) -> bool {
        match self {
            Filter::All => true,
            Filter::New => module.is_new(),
            Filter::Hud => module.category() == Category::Hud,
            Filter::Server => module.category() == Category::Server,
            Filter::Mechanic => module.category() == Category::Mechanic,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubTab {
    #[default]
    General,
    Controls,
}

impl SubTab {
    pub const ALL: [SubTab; 2] = [SubTab::General, SubTab::Controls];

    pub fn label(self) -> &'static str {
        match self {
            SubTab::General => "GENERAL",
            SubTab::Controls => "HUD LAYOUT",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Menu {
    pub open: bool,
    pub tab: Tab,
    pub filter: Filter,
    pub subtab: SubTab,
    pub search: String,
    /// The highlighted row: a card, a setting row, a waypoint or a layout row.
    pub selected: usize,
    /// The card whose OPTIONS page shows, as a registry index.
    pub options: Option<usize>,
    pub notice: Option<String>,
    /// Moving a HUD module's panel: the module it moves.
    pub editing: Option<usize>,
    /// Microseconds the last menu paint took, and the shapes it drew.
    pub paint_us: u32,
    pub paint_shapes: u32,
    pub paint_culled: u32,
}

/// Cards per page (three columns of four) and per row.
pub const COLS: usize = 3;
pub const ROWS: usize = 4;
pub const PER_PAGE: usize = COLS * ROWS;

impl Menu {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.options = None;
            self.notice = None;
            self.editing = None;
        }
    }

    /// What the player pressed; true means the config changed and must be saved.
    pub fn key(&mut self, registry: &mut Registry, key: &str) -> bool {
        if !self.open {
            return false;
        }
        let mut changed = false;
        match key {
            "key.keyboard.escape" => {
                if self.editing.take().is_none() && self.options.take().is_none() {
                    self.open = false;
                }
            }
            "key.keyboard.tab" => {
                self.tab = self.tab.next();
                self.selected = 0;
                self.options = None;
                self.editing = None;
            }
            "key.keyboard.up" => {
                if self.editing.is_some() {
                    self.move_panel(registry, 0.0, -2.0);
                    changed = true;
                } else {
                    self.selected = self.selected.saturating_sub(self.step_up());
                }
            }
            "key.keyboard.down" => {
                if self.editing.is_some() {
                    self.move_panel(registry, 0.0, 2.0);
                    changed = true;
                } else {
                    self.selected = (self.selected + self.step_up())
                        .min(self.rows(registry).saturating_sub(1));
                }
            }
            "key.keyboard.left" => {
                if self.editing.is_some() {
                    self.move_panel(registry, -2.0, 0.0);
                    changed = true;
                } else {
                    self.selected = self.selected.saturating_sub(1);
                }
            }
            "key.keyboard.right" => {
                if self.editing.is_some() {
                    self.move_panel(registry, 2.0, 0.0);
                    changed = true;
                } else {
                    self.selected = (self.selected + 1).min(self.rows(registry).saturating_sub(1));
                }
            }
            "key.keyboard.enter" | "key.keyboard.space" => changed = self.activate(registry),
            "key.keyboard.o" if self.tab == Tab::Mods && self.options.is_none() => {
                let visible = self.visible(registry);
                if let Some(&index) = visible.get(self.selected) {
                    self.options = Some(index);
                    self.selected = 0;
                }
            }
            "key.keyboard.h" if self.editing.is_none() => {
                self.start_editing(registry);
            }
            "key.keyboard.delete" if self.tab == Tab::Waypoints && self.editing.is_none() => {
                if let Some(waypoint) = registry.config.waypoints.get(self.selected).cloned() {
                    registry.config.delete_waypoint(&waypoint.name);
                    self.notice = Some(format!("{} deleted", waypoint.name));
                    self.selected = self
                        .selected
                        .min(registry.config.waypoints.len().saturating_sub(1));
                    changed = true;
                }
            }
            "key.keyboard.left.bracket" => {
                changed = registry.next_profile(-1);
                if changed {
                    self.notice = Some(format!("Profile: {}", registry.config.active_profile));
                }
            }
            "key.keyboard.right.bracket" => {
                changed = registry.next_profile(1);
                if changed {
                    self.notice = Some(format!("Profile: {}", registry.config.active_profile));
                }
            }
            _ => {}
        }
        changed
    }

    fn start_editing(&mut self, registry: &Registry) {
        let huds = registry.huds();
        if huds.is_empty() {
            self.notice = Some("No HUD modules are on".to_string());
            return;
        }
        let at = if self.tab == Tab::Mods && self.options.is_none() {
            let visible = self.visible(registry);
            visible
                .get(self.selected)
                .copied()
                .filter(|index| huds.contains(index))
        } else {
            None
        };
        let index = at.unwrap_or_else(|| {
            if self.subtab == SubTab::Controls {
                *huds.get(self.selected).unwrap_or(&huds[0])
            } else {
                huds[0]
            }
        });
        self.editing = Some(index);
        self.notice = Some("Arrows move the panel, Enter changes its corner".to_string());
    }

    fn move_panel(&mut self, registry: &mut Registry, dx: f32, dy: f32) {
        let Some(index) = self.editing else {
            return;
        };
        let id = registry.modules[index].id();
        let panel = hud_panel(registry, index, &HudInfo::default());
        let anchor = registry.config.anchor_mut(id);
        anchor.nudge(dx, dy, (1600.0 - panel.0, 900.0 - panel.1));
    }

    fn step_up(&self) -> usize {
        match self.tab {
            Tab::Mods if self.options.is_none() => COLS,
            _ => 1,
        }
    }

    /// Enter on the highlighted row.
    fn activate(&mut self, registry: &mut Registry) -> bool {
        if self.editing.is_some() {
            let Some(index) = self.editing else {
                return false;
            };
            let id = registry.modules[index].id();
            registry.config.anchor_mut(id).next_corner();
            return true;
        }
        match self.tab {
            Tab::Mods => {
                if let Some(index) = self.options {
                    let settings = registry.modules[index].settings();
                    if let Some(setting) = settings.get(self.selected).copied() {
                        registry.cycle_setting(index, &setting);
                        return true;
                    }
                } else {
                    let visible = self.visible(registry);
                    if let Some(&index) = visible.get(self.selected) {
                        let on = registry.toggle(index);
                        self.notice = Some(format!(
                            "{} {}",
                            registry.modules[index].name(),
                            if on { "ENABLED" } else { "DISABLED" }
                        ));
                        return true;
                    }
                }
                false
            }
            Tab::Settings => {
                if self.subtab == SubTab::Controls {
                    self.start_editing(registry);
                    return false;
                }
                let rows = self.setting_rows(registry);
                if let Some(&(module, setting)) = rows.get(self.selected) {
                    let s = registry.modules[module].settings()[setting];
                    registry.cycle_setting(module, &s);
                    return true;
                }
                false
            }
            Tab::Waypoints => {
                if let Some(waypoint) = registry.config.waypoints.get(self.selected) {
                    let name = waypoint.name.clone();
                    let tracking = registry.config.tracked.as_deref() == Some(name.as_str());
                    registry.config.track((!tracking).then_some(name.as_str()));
                    self.notice = Some(if tracking {
                        format!("{name} untracked")
                    } else {
                        format!("Tracking {name}")
                    });
                    return true;
                }
                false
            }
            Tab::Performance => false,
        }
    }

    /// The rows on the current screen, for Up/Down clamping.
    fn rows(&self, registry: &Registry) -> usize {
        match (self.tab, self.options) {
            (Tab::Mods, Some(index)) => registry.modules[index].settings().len(),
            (Tab::Mods, None) => self.visible(registry).len(),
            (Tab::Settings, _) if self.subtab == SubTab::Controls => registry.huds().len(),
            (Tab::Settings, _) => self.setting_rows(registry).len(),
            (Tab::Waypoints, _) => registry.config.waypoints.len(),
            (Tab::Performance, _) => 0,
        }
    }

    pub fn visible(&self, registry: &Registry) -> Vec<usize> {
        registry
            .modules
            .iter()
            .enumerate()
            .filter(|(_, m)| self.filter.matches(m.as_ref()))
            .filter(|(_, m)| {
                let q = self.search.trim().to_lowercase();
                q.is_empty()
                    || m.name().to_lowercase().contains(&q)
                    || m.id().to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn setting_rows(&self, registry: &Registry) -> Vec<(usize, usize)> {
        if self.subtab != SubTab::General {
            return Vec::new();
        }
        let q = self.search.trim().to_lowercase();
        let mut rows = Vec::new();
        for (i, m) in registry.modules.iter().enumerate() {
            if !(q.is_empty() || m.name().to_lowercase().contains(&q)) {
                continue;
            }
            for s in 0..m.settings().len() {
                rows.push((i, s));
            }
        }
        rows
    }

    pub fn page(&self, _registry: &Registry) -> usize {
        if self.tab == Tab::Mods && self.options.is_none() {
            self.selected / PER_PAGE
        } else {
            0
        }
    }

    /// Commands: `/morinume ...`. Returns the chat feedback.
    pub fn command(
        &mut self,
        registry: &mut Registry,
        args: &str,
        at: Option<(f64, f64, f64, String)>,
    ) -> Result<Option<String>, String> {
        let mut words = args.split_whitespace();
        match words.next() {
            None | Some("open" | "menu") => {
                self.open = true;
                self.options = None;
                Ok(None)
            }
            Some("close") => {
                self.open = false;
                Ok(Some("Morinume closed".to_string()))
            }
            Some("toggle") => {
                let id = words.next().ok_or("Usage: /morinume toggle <module id>")?;
                let index = registry
                    .modules
                    .iter()
                    .position(|m| m.id() == id)
                    .ok_or_else(|| format!("No module `{id}`; try /morinume list"))?;
                let on = registry.toggle(index);
                Ok(Some(format!(
                    "{} {}",
                    registry.modules[index].name(),
                    if on { "enabled" } else { "disabled" }
                )))
            }
            Some("search") => {
                self.search = words.collect::<Vec<_>>().join(" ");
                self.selected = 0;
                Ok(Some(if self.search.is_empty() {
                    "Search cleared".to_string()
                } else {
                    format!("Search: {}", self.search)
                }))
            }
            Some("edit") => {
                self.open = true;
                self.tab = Tab::Settings;
                self.subtab = SubTab::Controls;
                self.start_editing(registry);
                Ok(self.notice.clone())
            }
            Some("perf") => {
                self.open = true;
                self.tab = Tab::Performance;
                self.options = None;
                Ok(None)
            }
            Some("profile") => self.profile_command(registry, words.collect()),
            Some("waypoint") => self.waypoint_command(registry, words.collect(), at),
            Some("save") => Ok(Some(match registry.config.save_as_new() {
                Some(name) => format!("Saved as {name}"),
                None => "32 profiles is the limit".to_string(),
            })),
            Some("list") => {
                let names: Vec<String> = registry
                    .modules
                    .iter()
                    .enumerate()
                    .map(|(i, m)| {
                        format!(
                            "{} ({})",
                            m.id(),
                            if registry.enabled(i) { "on" } else { "off" }
                        )
                    })
                    .collect();
                Ok(Some(names.join(", ")))
            }
            Some(other) => Err(format!("Unknown subcommand `{other}`")),
        }
    }

    fn profile_command(
        &mut self,
        registry: &mut Registry,
        words: Vec<&str>,
    ) -> Result<Option<String>, String> {
        match words.first().copied() {
            None | Some("list") => Ok(Some(format!(
                "Profile {} (of {})",
                registry.config.active_profile,
                registry.config.profile_names().join(", ")
            ))),
            Some("use") => {
                let name = words.get(1).ok_or("Usage: /morinume profile use <name>")?;
                if !registry.activate_profile(name) {
                    return Err(format!("No profile `{name}`"));
                }
                Ok(Some(format!("Profile: {name}")))
            }
            Some("new") => {
                let name = words.get(1).ok_or("Usage: /morinume profile new <name>")?;
                if !registry.config.create_profile(name) {
                    return Err(format!("Cannot create `{name}`"));
                }
                Ok(Some(format!("Profile: {name}")))
            }
            Some("delete") => {
                let name = words.get(1).ok_or("Usage: /morinume profile delete <name>")?;
                if !registry.config.delete_profile(name) {
                    return Err(format!("Cannot delete `{name}`"));
                }
                registry.sync();
                Ok(Some(format!("Deleted {name}")))
            }
            Some(other) => Err(format!("Unknown profile command `{other}`")),
        }
    }

    fn waypoint_command(
        &mut self,
        registry: &mut Registry,
        words: Vec<&str>,
        at: Option<(f64, f64, f64, String)>,
    ) -> Result<Option<String>, String> {
        match words.first().copied() {
            None | Some("list") => {
                if registry.config.waypoints.is_empty() {
                    return Ok(Some("No waypoints".to_string()));
                }
                let list: Vec<String> = registry
                    .config
                    .waypoints
                    .iter()
                    .map(|w| format!("{} ({:.0}, {:.0}, {:.0})", w.name, w.x, w.y, w.z))
                    .collect();
                Ok(Some(list.join(", ")))
            }
            Some("add") => {
                let name = words.get(1).ok_or("Usage: /morinume waypoint add <name> [x y z]")?;
                let position = match (words.get(2), words.get(3), words.get(4)) {
                    (Some(x), Some(y), Some(z)) => {
                        let parsed = (x.parse().ok(), y.parse().ok(), z.parse().ok());
                        let (Some(x), Some(y), Some(z)) = parsed else {
                            return Err("Coordinates must be numbers".to_string());
                        };
                        (x, y, z, at.as_ref().map(|a| a.3.clone()).unwrap_or_else(|| "minecraft:overworld".to_string()))
                    }
                    _ => at.ok_or("No player position in this context")?,
                };
                let waypoint = Waypoint {
                    name: name.to_string(),
                    x: position.0,
                    y: position.1,
                    z: position.2,
                    dim: position.3,
                };
                if !registry.config.add_waypoint(waypoint) {
                    return Err("Waypoint not added (128 is the limit)".to_string());
                }
                Ok(Some(format!("Waypoint {name}")))
            }
            Some("del" | "delete") => {
                let name = words.get(1).ok_or("Usage: /morinume waypoint del <name>")?;
                if !registry.config.delete_waypoint(name) {
                    return Err(format!("No waypoint `{name}`"));
                }
                Ok(Some(format!("Deleted {name}")))
            }
            Some("track") => {
                let name = words.get(1).copied();
                if !registry.config.track(name) {
                    return Err(format!("No waypoint `{}`", name.unwrap_or("")));
                }
                Ok(Some(match name {
                    None | Some("none") => "Waypoint untracked".to_string(),
                    Some(name) => format!("Tracking {name}"),
                }))
            }
            Some(other) => Err(format!("Unknown waypoint command `{other}`")),
        }
    }
}

// ---------------------------------------------------------------------------
// Painting (pure data; `hooks.rs` turns it into HUD shapes)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        argb: u32,
    },
    Text {
        text: String,
        x: f32,
        y: f32,
        rgb: u32,
        shadow: bool,
    },
}

const BG: u32 = 0xE60D0E12;
const PANEL: u32 = 0xF0161920;
const CARD: u32 = 0xFF1E222B;
const CARD_SEL: u32 = 0xFF262C38;
const LINE: u32 = 0xFF2C3240;
const BLUE: u32 = 0xFF5B9BD5;
const GOLD: u32 = 0xFFE0B45A;
const GREEN: u32 = 0xFF3FBF6F;
const RED: u32 = 0xFFCF5A5A;
const WHITE: u32 = 0xFFF2F4F8;
const MUTED: u32 = 0xFF9AA0A6;
const DIM: u32 = 0xFF6E7681;
const HUD_BG: u32 = 0x9E0D0E12;
const CHIP: u32 = 0xFF2A2F3A;

const PAD: f32 = 10.0;
const TOP: f32 = 30.0;
const BAR_H: f32 = 14.0;
/// The GUI font's average advance at scale 1, for panel backgrounds.
const CHAR_W: f32 = 6.0;

struct Painter {
    out: Vec<Paint>,
}

impl Painter {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, argb: u32) {
        self.out.push(Paint::Rect { x, y, w, h, argb });
    }
    fn text(&mut self, text: impl Into<String>, x: f32, y: f32, rgb: u32) {
        self.out.push(Paint::Text {
            text: text.into(),
            x,
            y,
            rgb,
            shadow: false,
        });
    }
    fn outline(&mut self, x: f32, y: f32, w: f32, h: f32, argb: u32) {
        self.rect(x, y, w, 1.0, argb);
        self.rect(x, y + h - 1.0, w, 1.0, argb);
        self.rect(x, y, 1.0, h, argb);
        self.rect(x + w - 1.0, y, 1.0, h, argb);
    }
}

fn text_w(text: &str) -> f32 {
    text.chars().count() as f32 * CHAR_W
}

/// A HUD panel's size: its lines and its key chips.
pub fn hud_panel(registry: &Registry, index: usize, info: &HudInfo) -> (f32, f32) {
    let module = &registry.modules[index];
    let lines = module.hud(info, &registry.config);
    let chips = module.chips(info, &registry.config);
    let mut w: f32 = 24.0;
    for line in &lines {
        w = w.max(text_w(line) + 10.0);
    }
    if !chips.is_empty() {
        w = w.max(chips.len().min(4) as f32 * 17.0 + 8.0);
    }
    let h = lines.len() as f32 * 10.0 + chips.len().div_ceil(4) as f32 * 15.0 + 8.0;
    (w.max(28.0), h.max(16.0))
}

impl Menu {
    /// The whole menu as fills and text, in GUI pixels, for a `w` x `h` screen.
    pub fn paint(&self, registry: &Registry, w: f32, h: f32) -> Vec<Paint> {
        let mut p = Painter { out: Vec::new() };
        p.rect(0.0, 0.0, w, h, BG);
        let pw = (w * 0.88).min(760.0);
        let ph = (h * 0.84).min(520.0);
        let px = ((w - pw) / 2.0).floor();
        let py = ((h - ph) / 2.0).floor();
        p.rect(px, py, pw, ph, PANEL);
        p.outline(px, py, pw, ph, LINE);
        self.top_bar(&mut p, registry, px, py, pw);
        match (self.tab, self.options) {
            (_, Some(index)) => self.options_page(&mut p, registry, index, px, py, pw, ph),
            (Tab::Mods, None) => self.mods_page(&mut p, registry, px, py, pw, ph),
            (Tab::Settings, None) => self.settings_page(&mut p, registry, px, py, pw, ph),
            (Tab::Waypoints, None) => self.waypoints_page(&mut p, registry, px, py, pw, ph),
            (Tab::Performance, None) => self.performance_page(&mut p, registry, px, py, pw, ph),
        }
        if self.editing.is_some() {
            p.rect(px, py + ph - 34.0, pw, 20.0, CARD);
            p.text(
                "EDIT HUD: arrows move   enter corner   esc done",
                px + PAD,
                py + ph - 28.0,
                GOLD,
            );
        }
        p.out
    }

    fn top_bar(&self, p: &mut Painter, registry: &Registry, px: f32, py: f32, pw: f32) {
        p.rect(px, py, pw, TOP, 0xFF12151B);
        p.rect(px + PAD, py + 8.0, 12.0, 12.0, GOLD);
        p.text("MORINUME", px + PAD + 18.0, py + 10.0, WHITE);
        let mut x = px + pw / 2.0 - 160.0;
        for tab in Tab::ALL {
            let active = tab == self.tab;
            p.text(
                tab.label(),
                x,
                py + 10.0,
                if active { BLUE } else { MUTED },
            );
            if active {
                p.rect(x, py + TOP - 3.0, text_w(tab.label()), 2.0, BLUE);
            }
            x += text_w(tab.label()) + 16.0;
        }
        p.text("X", px + pw - PAD - 6.0, py + 10.0, MUTED);
        p.text(
            format!("{} profile", registry.config.active_profile),
            px + PAD,
            py + TOP + 6.0,
            DIM,
        );
        p.rect(px, py + TOP, pw, 1.0, LINE);
    }

    fn mods_page(
        &self,
        p: &mut Painter,
        registry: &Registry,
        px: f32,
        py: f32,
        pw: f32,
        ph: f32,
    ) {
        // Sidebar: profiles.
        let side = 132.0;
        p.rect(px, py + TOP + 1.0, side, ph - TOP - 1.0, 0xFF12151B);
        let names = registry.config.profile_names();
        p.text("PROFILES  [ ]", px + PAD, py + TOP + 24.0, MUTED);
        let mut y = py + TOP + 40.0;
        for name in &names {
            let active = *name == registry.config.active_profile;
            p.text(
                if active {
                    format!("> {name}")
                } else {
                    format!("  {name}")
                },
                px + PAD,
                y,
                if active { WHITE } else { MUTED },
            );
            y += 12.0;
        }
        p.rect(px + PAD, py + ph - 56.0, side - 2.0 * PAD, 18.0, CARD);
        p.text("SAVE AS NEW", px + PAD + 4.0, py + ph - 51.0, MUTED);
        p.rect(px + PAD, py + ph - 32.0, side - 2.0 * PAD, 18.0, CARD);
        p.text("EDIT HUD  [H]", px + PAD + 4.0, py + ph - 27.0, MUTED);

        // Filter chips + search.
        let gx = px + side + PAD;
        let gw = pw - side - 2.0 * PAD;
        let mut x = gx;
        for filter in Filter::ALL {
            let active = filter == self.filter;
            let w = 12.0 + filter.label().len() as f32 * 6.0;
            p.rect(x, py + TOP + 8.0, w, 14.0, if active { BLUE } else { CARD });
            p.text(
                filter.label(),
                x + 6.0,
                py + TOP + 11.0,
                if active { 0xFF0D0E12 } else { MUTED },
            );
            x += w + 4.0;
        }
        p.text(
            if self.search.is_empty() {
                "SEARCH".to_string()
            } else {
                format!("SEARCH: {}", self.search)
            },
            gx + gw - 96.0,
            py + TOP + 11.0,
            DIM,
        );

        // Cards: three columns, four rows, one page.
        let visible = self.visible(registry);
        let page = self.page(registry);
        let slice: Vec<usize> = visible
            .iter()
            .skip(page * PER_PAGE)
            .take(PER_PAGE)
            .copied()
            .collect();
        let top = py + TOP + 30.0;
        let card_h = ((ph - TOP - 56.0) / ROWS as f32).floor().max(64.0) - 6.0;
        let card_w = ((gw - (COLS as f32 - 1.0) * 8.0) / COLS as f32).floor();
        for (slot, &index) in slice.iter().enumerate() {
            let col = slot % COLS;
            let row = slot / COLS;
            let x = gx + col as f32 * (card_w + 8.0);
            let y = top + row as f32 * (card_h + 6.0);
            let module = &registry.modules[index];
            let selected = page * PER_PAGE + slot == self.selected;
            p.rect(x, y, card_w, card_h, if selected { CARD_SEL } else { CARD });
            if selected {
                p.outline(x, y, card_w, card_h, BLUE);
            }
            p.rect(x + 8.0, y + 8.0, 14.0, 14.0, module.accent() | 0xFF000000);
            p.text(module.name(), x + 28.0, y + 11.0, WHITE);
            if module.is_new() {
                p.text("NEW", x + card_w - 30.0, y + 11.0, GOLD);
            }
            p.text(module.category().label(), x + 8.0, y + 30.0, DIM);
            p.text("OPTIONS", x + 8.0, y + card_h - 26.0, MUTED);
            let on = registry.enabled(index);
            p.rect(
                x,
                y + card_h - BAR_H,
                card_w,
                BAR_H,
                if on { GREEN } else { RED },
            );
            let label = if on { "ENABLED" } else { "DISABLED" };
            p.text(label, x + card_w / 2.0 - 16.0, y + card_h - BAR_H + 3.0, 0xFF0D0E12);
        }
        if visible.is_empty() {
            p.text("No mods match this filter.", gx, top + 4.0, MUTED);
        }
        if visible.len() > PER_PAGE {
            p.text(
                format!("PAGE {}/{}", page + 1, visible.len().div_ceil(PER_PAGE)),
                gx,
                py + ph - 18.0,
                DIM,
            );
        }
        if let Some(notice) = &self.notice {
            p.text(notice, gx, py + ph - 18.0, MUTED);
        }
        self.footer(p, px, py, ph, pw);
    }

    fn settings_page(
        &self,
        p: &mut Painter,
        registry: &Registry,
        px: f32,
        py: f32,
        pw: f32,
        ph: f32,
    ) {
        let mut x = px + PAD;
        for subtab in SubTab::ALL {
            let active = subtab == self.subtab;
            let w = 12.0 + subtab.label().len() as f32 * 6.0;
            p.rect(x, py + TOP + 8.0, w, 14.0, if active { BLUE } else { CARD });
            p.text(
                subtab.label(),
                x + 6.0,
                py + TOP + 11.0,
                if active { 0xFF0D0E12 } else { MUTED },
            );
            x += w + 4.0;
        }
        if self.subtab == SubTab::Controls {
            self.layout_rows(p, registry, px, py, pw, ph);
            self.footer(p, px, py, ph, pw);
            return;
        }
        let rows = self.setting_rows(registry);
        let mut y = py + TOP + 34.0;
        let mut last: Option<Category> = None;
        for (row, &(module, setting)) in rows.iter().enumerate() {
            let m = &registry.modules[module];
            if last != Some(m.category()) {
                last = Some(m.category());
                p.text(m.category().label(), px + PAD, y, BLUE);
                y += 14.0;
            }
            let selected = row == self.selected;
            p.rect(px + PAD, y - 1.0, pw - 2.0 * PAD, 13.0, if selected { CARD_SEL } else { CARD });
            p.text(
                format!(
                    "{}  {}",
                    m.name(),
                    registry.setting_label(module, &m.settings()[setting])
                ),
                px + PAD + 6.0,
                y + 2.0,
                if selected { WHITE } else { MUTED },
            );
            y += 16.0;
            if y > py + ph - 26.0 {
                break;
            }
        }
        if rows.is_empty() {
            p.text("No settings yet.", px + PAD, py + TOP + 40.0, MUTED);
        }
        self.footer(p, px, py, ph, pw);
    }

    fn layout_rows(
        &self,
        p: &mut Painter,
        registry: &Registry,
        px: f32,
        py: f32,
        _pw: f32,
        _ph: f32,
    ) {
        let huds = registry.huds();
        let mut y = py + TOP + 34.0;
        if huds.is_empty() {
            p.text("No HUD modules are on.", px + PAD, y, MUTED);
            return;
        }
        for (row, &index) in huds.iter().enumerate() {
            let module = &registry.modules[index];
            let anchor = registry.config.anchor(module.id());
            let selected = row == self.selected;
            p.rect(
                px + PAD,
                y - 1.0,
                430.0,
                15.0,
                if selected { CARD_SEL } else { CARD },
            );
            p.text(
                format!(
                    "{}  {}  {},{}",
                    module.name(),
                    anchor.corner,
                    anchor.x as i32,
                    anchor.y as i32
                ),
                px + PAD + 6.0,
                y + 3.0,
                if selected { WHITE } else { MUTED },
            );
            y += 18.0;
        }
        p.text(
            "enter edits the selected panel",
            px + PAD,
            y + 4.0,
            DIM,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn options_page(
        &self,
        p: &mut Painter,
        registry: &Registry,
        index: usize,
        px: f32,
        py: f32,
        pw: f32,
        ph: f32,
    ) {
        let module = &registry.modules[index];
        p.text(module.name(), px + PAD, py + TOP + 12.0, WHITE);
        p.text(module.category().label(), px + PAD, py + TOP + 26.0, MUTED);
        let mut y = py + TOP + 48.0;
        for (row, setting) in module.settings().iter().enumerate() {
            let selected = row == self.selected;
            p.rect(px + PAD, y - 1.0, pw - 2.0 * PAD, 16.0, if selected { CARD_SEL } else { CARD });
            p.text(
                registry.setting_label(index, setting),
                px + PAD + 6.0,
                y + 3.0,
                if selected { WHITE } else { MUTED },
            );
            y += 18.0;
        }
        if module.settings().is_empty() {
            p.text("No options.", px + PAD, y, MUTED);
        }
        let state = if registry.enabled(index) {
            "ENABLED"
        } else {
            "DISABLED"
        };
        let status = match module.status() {
            Some(extra) => format!("{state}  |  {extra}"),
            None => state.to_string(),
        };
        p.text(status, px + PAD, py + ph - 34.0, MUTED);
        p.rect(px + PAD, py + ph - 22.0, 70.0, 14.0, CARD);
        p.text("BACK", px + PAD + 22.0, py + ph - 19.0, MUTED);
        self.footer(p, px, py, ph, pw);
    }

    fn waypoints_page(
        &self,
        p: &mut Painter,
        registry: &Registry,
        px: f32,
        py: f32,
        pw: f32,
        ph: f32,
    ) {
        let list = &registry.config.waypoints;
        p.text(
            format!(
                "{} WAYPOINTS   enter tracks   delete removes",
                list.len()
            ),
            px + PAD,
            py + TOP + 12.0,
            MUTED,
        );
        let mut y = py + TOP + 34.0;
        for (row, waypoint) in list.iter().enumerate() {
            let selected = row == self.selected;
            let tracked = registry.config.tracked.as_deref() == Some(waypoint.name.as_str());
            p.rect(px + PAD, y - 1.0, pw - 2.0 * PAD, 16.0, if selected { CARD_SEL } else { CARD });
            p.text(
                format!(
                    "{} {}  {:.0}, {:.0}, {:.0}  {}",
                    if tracked { ">" } else { " " },
                    waypoint.name,
                    waypoint.x,
                    waypoint.y,
                    waypoint.z,
                    waypoint.dim.rsplit(':').next().unwrap_or("")
                ),
                px + PAD + 6.0,
                y + 3.0,
                if tracked {
                    GOLD
                } else if selected {
                    WHITE
                } else {
                    MUTED
                },
            );
            y += 18.0;
            if y > py + ph - 28.0 {
                break;
            }
        }
        if list.is_empty() {
            p.text(
                "/morinume waypoint add <name> adds one here",
                px + PAD + 6.0,
                y + 4.0,
                DIM,
            );
        }
        if let Some(notice) = &self.notice {
            p.text(notice, px + PAD, py + ph - 18.0, MUTED);
        }
        self.footer(p, px, py, ph, pw);
    }

    fn performance_page(
        &self,
        p: &mut Painter,
        registry: &Registry,
        px: f32,
        py: f32,
        pw: f32,
        ph: f32,
    ) {
        let (enabled, total) = registry.enabled_count();
        let huds = registry.huds().len();
        p.text("PERFORMANCE", px + PAD, py + TOP + 12.0, WHITE);
        p.text(
            "measured live, by Morinume and the game's own counters",
            px + PAD,
            py + TOP + 26.0,
            DIM,
        );
        let rows: Vec<(String, String)> = vec![
            (
                "Menu paint".to_string(),
                if self.paint_us == 0 {
                    "not measured yet".to_string()
                } else {
                    format!("{} us", self.paint_us)
                },
            ),
            (
                "Shapes drawn".to_string(),
                format!("{} ({} culled)", self.paint_shapes, self.paint_culled),
            ),
            ("Modules".to_string(), format!("{enabled}/{total} on, {huds} HUD")),
            (
                "Config".to_string(),
                format!("{} bytes", registry.config.to_json().len()),
            ),
            (
                "Profiles".to_string(),
                format!(
                    "{} of {MAX_PROFILES}",
                    registry.config.profile_names().len()
                ),
            ),
            (
                "Waypoints".to_string(),
                format!(
                    "{} of {MAX_WAYPOINTS}",
                    registry.config.waypoints.len()
                ),
            ),
        ];
        let mut y = py + TOP + 46.0;
        for (label, value) in rows {
            p.rect(px + PAD, y - 1.0, pw - 2.0 * PAD, 16.0, CARD);
            p.text(label, px + PAD + 6.0, y + 3.0, MUTED);
            p.text(value, px + PAD + 180.0, y + 3.0, WHITE);
            y += 18.0;
        }
        p.text(
            "FPS, min and 1% are on the FPS Counter's HUD panel",
            px + PAD,
            y + 6.0,
            DIM,
        );
        self.footer(p, px, py, ph, pw);
    }

    fn footer(&self, p: &mut Painter, px: f32, py: f32, ph: f32, pw: f32) {
        p.rect(px, py + ph - 14.0, pw, 1.0, LINE);
        p.text(
            "ARROWS MOVE   ENTER SELECT   O OPTIONS   H EDIT HUD   TAB TAB   ESC CLOSE",
            px + PAD,
            py + ph - 11.0,
            DIM,
        );
    }
}

/// The HUD panels of the enabled modules, as fills and text.
pub fn paint_hud(registry: &Registry, info: &HudInfo, w: f32, h: f32) -> Vec<Paint> {
    let mut p = Painter { out: Vec::new() };
    for index in registry.huds() {
        let module = &registry.modules[index];
        let lines = module.hud(info, &registry.config);
        let chips = module.chips(info, &registry.config);
        if lines.is_empty() && chips.is_empty() {
            continue;
        }
        let (pw, ph) = hud_panel(registry, index, info);
        let (x, y) = registry.config.anchor(module.id()).at(pw, ph, w, h);
        p.rect(x, y, pw, ph, HUD_BG);
        p.outline(x, y, pw, ph, 0x552C3240);
        let mut ty = y + 4.0;
        for line in &lines {
            p.text(line, x + 5.0, ty, WHITE);
            ty += 10.0;
        }
        if !chips.is_empty() {
            for (row, chunk) in chips.chunks(4).enumerate() {
                let mut cx = x + 5.0;
                let cy = y + 4.0 + lines.len() as f32 * 10.0 + row as f32 * 15.0;
                for chip in chunk {
                    p.rect(cx, cy, 15.0, 13.0, if chip.pressed { GREEN } else { CHIP });
                    p.text(
                        chip.label,
                        cx + (15.0 - text_w(chip.label)) / 2.0,
                        cy + 2.0,
                        if chip.pressed { 0xFF0D0E12 } else { MUTED },
                    );
                    cx += 17.0;
                }
            }
        }
    }
    p.out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry::new(Config::default())
    }

    fn info() -> HudInfo {
        HudInfo {
            fps: 120,
            slowest: 95,
            low: 88,
            playtime: 20 * 3700,
            x: 12.7,
            y: 64.0,
            z: -3.2,
            yaw: 180.0,
            dimension: "minecraft:overworld".to_string(),
            cps: 7,
            keys: Keys {
                forward: true,
                ..Keys::default()
            },
            menu_open: false,
        }
    }

    #[test]
    fn the_registry_has_the_release_modules_and_defaults() {
        let registry = registry();
        let ids: Vec<&str> = registry.modules.iter().map(|m| m.id()).collect();
        assert_eq!(
            ids,
            ["fps", "cps", "keystrokes", "coords", "playtime", "tracker", "performance"]
        );
        assert!(registry.enabled(0) && registry.enabled(1) && registry.enabled(2));
        assert!(!registry.enabled(4));
        assert!(registry.enabled(6));
    }

    #[test]
    fn toggling_saves_the_override_in_the_active_profile() {
        let mut registry = registry();
        assert!(!registry.toggle(0));
        assert!(!registry.enabled(0));
        assert_eq!(
            registry
                .config
                .profiles
                .get(DEFAULT_PROFILE)
                .unwrap()
                .get("fps"),
            Some(&false)
        );
        assert!(registry.toggle(0));
    }

    #[test]
    fn config_survives_json_round_trips() {
        let mut config = Config::default();
        config.set_enabled("fps", false);
        config.set_setting("fps", "slowest", "off");
        config.hud.insert(
            "cps".into(),
            Anchor {
                corner: "bottom-right".into(),
                x: 2.0,
                y: 30.0,
            },
        );
        config.add_waypoint(Waypoint {
            name: "Base".into(),
            x: 10.0,
            y: 70.0,
            z: -4.0,
            dim: "minecraft:overworld".into(),
        });
        assert!(config.track(Some("Base")));
        let text = config.to_json();
        let back = Config::from_json(&text);
        assert_eq!(config, back);
        assert!(!back.is_enabled("fps", true));
        assert_eq!(back.setting("fps", "slowest"), Some("off"));
        assert_eq!(back.anchor("cps").corner, "bottom-right");
        assert_eq!(back.tracked.as_deref(), Some("Base"));
    }

    #[test]
    fn a_broken_config_falls_back_to_defaults() {
        let config = Config::from_json("not json");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn profiles_start_as_a_copy_and_have_unique_names() {
        let mut config = Config::default();
        config.set_enabled("fps", false);
        assert_eq!(config.save_as_new().as_deref(), Some("Profile 2"));
        assert!(!config.is_enabled("fps", true));
        config.set_enabled("fps", true);
        assert_eq!(config.save_as_new().as_deref(), Some("Profile 3"));
        assert!(config.activate(DEFAULT_PROFILE));
        assert!(!config.is_enabled("fps", true));
        assert!(!config.activate("Nope"));
    }

    #[test]
    fn profiles_can_be_named_cycled_and_deleted() {
        let mut registry = registry();
        assert!(registry.activate_profile(DEFAULT_PROFILE));
        assert!(registry.config.create_profile("PvP"));
        assert_eq!(registry.config.active_profile, "PvP");
        assert!(!registry.config.create_profile("PvP"));
        assert!(!registry.config.delete_profile(DEFAULT_PROFILE));
        registry.toggle(0);
        assert!(registry.next_profile(1));
        assert_eq!(registry.config.active_profile, DEFAULT_PROFILE);
        assert!(registry.enabled(0));
        assert!(registry.config.delete_profile("PvP"));
    }

    #[test]
    fn waypoints_add_track_and_measure() {
        let mut config = Config::default();
        assert!(config.add_waypoint(Waypoint {
            name: "Base".into(),
            x: 3.0,
            y: 64.0,
            z: 4.0,
            dim: "minecraft:overworld".into(),
        }));
        assert!(config.add_waypoint(Waypoint {
            name: "Base".into(),
            x: 0.0,
            y: 64.0,
            z: 0.0,
            dim: "minecraft:overworld".into(),
        }));
        assert_eq!(config.waypoints.len(), 1);
        assert_eq!(config.waypoints[0].x, 0.0);
        assert!(config.track(Some("Base")));
        assert_eq!(
            config.tracked_waypoint().unwrap().distance(3.0, 4.0, "minecraft:overworld"),
            Some(5.0)
        );
        assert_eq!(
            config.tracked_waypoint().unwrap().distance(3.0, 4.0, "minecraft:the_nether"),
            None
        );
        assert!(config.delete_waypoint("Base"));
        assert!(config.tracked.is_none());
        assert!(!config.delete_waypoint("Base"));
    }

    #[test]
    fn filters_and_search_narrow_the_cards() {
        let registry = registry();
        let mut menu = Menu::default();
        assert_eq!(menu.visible(&registry).len(), 7);
        menu.filter = Filter::Hud;
        assert_eq!(menu.visible(&registry).len(), 6);
        menu.filter = Filter::New;
        assert_eq!(menu.visible(&registry), vec![0, 2, 6]);
        menu.filter = Filter::All;
        menu.search = "zzz".into();
        assert!(menu.visible(&registry).is_empty());
        menu.search = "coord".into();
        assert_eq!(menu.visible(&registry), vec![3]);
    }

    #[test]
    fn enter_toggles_the_selected_card_and_escape_closes() {
        let mut registry = registry();
        let mut menu = Menu {
            open: true,
            ..Menu::default()
        };
        assert!(menu.key(&mut registry, "key.keyboard.enter"));
        assert!(!registry.enabled(0));
        menu.key(&mut registry, "key.keyboard.escape");
        assert!(!menu.open);
    }

    #[test]
    fn o_opens_the_options_page_and_enter_cycles_its_rows() {
        let mut registry = registry();
        let mut menu = Menu {
            open: true,
            ..Menu::default()
        };
        menu.key(&mut registry, "key.keyboard.o");
        assert_eq!(menu.options, Some(0));
        assert!(menu.key(&mut registry, "key.keyboard.enter"));
        assert_eq!(registry.config.setting("fps", "slowest"), Some("off"));
        menu.key(&mut registry, "key.keyboard.escape");
        assert_eq!(menu.options, None);
        assert!(menu.open);
    }

    #[test]
    fn tab_cycles_the_four_tabs() {
        let mut registry = registry();
        let mut menu = Menu {
            open: true,
            ..Menu::default()
        };
        for tab in [Tab::Settings, Tab::Waypoints, Tab::Performance, Tab::Mods] {
            menu.key(&mut registry, "key.keyboard.tab");
            assert_eq!(menu.tab, tab);
        }
    }

    #[test]
    fn commands_toggle_search_and_list() {
        let mut registry = registry();
        let mut menu = Menu::default();
        menu.command(&mut registry, "toggle fps", None).unwrap();
        assert!(!registry.enabled(0));
        menu.command(&mut registry, "search fps", None).unwrap();
        assert_eq!(menu.search, "fps");
        assert!(menu.command(&mut registry, "toggle nope", None).is_err());
        let list = menu
            .command(&mut registry, "list", None)
            .unwrap()
            .unwrap();
        assert!(list.contains("fps (off)"));
    }

    #[test]
    fn waypoint_commands_add_track_and_delete() {
        let mut registry = registry();
        let mut menu = Menu::default();
        let player = (10.5, 64.0, -8.25, "minecraft:overworld".to_string());
        menu.command(&mut registry, "waypoint add Base", Some(player.clone()))
            .unwrap();
        assert_eq!(registry.config.waypoints[0].x, 10.5);
        assert_eq!(registry.config.waypoints[0].dim, "minecraft:overworld");
        menu.command(&mut registry, "waypoint track Base", Some(player.clone()))
            .unwrap();
        assert_eq!(registry.config.tracked.as_deref(), Some("Base"));
        let list = menu
            .command(&mut registry, "waypoint list", None)
            .unwrap()
            .unwrap();
        assert!(list.contains("Base"));
        menu.command(&mut registry, "waypoint del Base", None).unwrap();
        assert!(registry.config.waypoints.is_empty());
        assert!(menu.command(&mut registry, "waypoint track nope", None).is_err());
    }

    #[test]
    fn profile_commands_create_use_and_delete() {
        let mut registry = registry();
        let mut menu = Menu::default();
        menu.command(&mut registry, "profile new PvP", None).unwrap();
        assert_eq!(registry.config.active_profile, "PvP");
        menu.command(&mut registry, "profile use Default", None)
            .unwrap();
        assert_eq!(registry.config.active_profile, DEFAULT_PROFILE);
        menu.command(&mut registry, "profile delete PvP", None).unwrap();
        assert!(!registry.config.profiles.contains_key("PvP"));
    }

    #[test]
    fn hud_modules_show_their_lines() {
        let registry = registry();
        let info = info();
        let ids: Vec<&str> = registry.modules.iter().map(|m| m.id()).collect();
        let lines = |id: &str| {
            let at = ids.iter().position(|x| *x == id).unwrap();
            registry.modules[at].hud(&info, &registry.config)
        };
        assert_eq!(lines("fps"), ["120 FPS", "min 95", "1% 88"]);
        assert_eq!(lines("cps"), ["7 CPS"]);
        assert_eq!(lines("coords")[0], "XYZ 12 64 -4");
        assert_eq!(lines("coords")[1], "facing north");
        assert_eq!(lines("playtime"), ["1:01:40"]);
        assert_eq!(lines("tracker"), ["no waypoint"]);
    }

    #[test]
    fn settings_change_what_a_hud_shows() {
        let mut registry = registry();
        registry.config.set_setting("fps", "slowest", "off");
        registry.config.set_setting("coords", "dimension", "on");
        registry.config.set_setting("coords", "facing", "off");
        let info = info();
        let fps = registry.modules[0].hud(&info, &registry.config);
        assert_eq!(fps, ["120 FPS", "1% 88"]);
        let coords = registry.modules[3].hud(&info, &registry.config);
        assert_eq!(coords, ["XYZ 12 64 -4", "overworld"]);
    }

    #[test]
    fn keystrokes_show_pressed_chips_and_follow_settings() {
        let mut registry = registry();
        let info = info();
        let chips = registry.modules[2].chips(&info, &registry.config);
        assert!(chips.iter().any(|c| c.label == "W" && c.pressed));
        assert!(chips.iter().any(|c| c.label == "LMB"));
        registry.config.set_setting("keystrokes", "mouse", "off");
        let chips = registry.modules[2].chips(&info, &registry.config);
        assert!(!chips.iter().any(|c| c.label == "LMB"));
    }

    #[test]
    fn the_tracker_follows_a_waypoint_in_its_dimension() {
        let mut registry = registry();
        let info = info();
        registry.config.add_waypoint(Waypoint {
            name: "Base".into(),
            x: 15.0,
            y: 64.0,
            z: -3.0,
            dim: "minecraft:overworld".into(),
        });
        registry.config.track(Some("Base"));
        let lines = registry.modules[5].hud(&info, &registry.config);
        assert_eq!(lines[0], "Base");
        assert!(lines[1].ends_with(" m"));
    }

    #[test]
    fn anchors_place_panels_in_their_corner() {
        let mut anchor = Anchor {
            corner: "bottom-right".into(),
            x: 4.0,
            y: 6.0,
        };
        assert_eq!(anchor.at(20.0, 10.0, 100.0, 50.0), (76.0, 34.0));
        anchor.corner = "top-left".into();
        assert_eq!(anchor.at(20.0, 10.0, 100.0, 50.0), (4.0, 6.0));
        anchor.next_corner();
        assert_eq!(anchor.corner, "top-right");
        anchor.nudge(2.0, 3.0, (100.0, 50.0));
        assert_eq!((anchor.x, anchor.y), (6.0, 9.0));
    }

    #[test]
    fn the_hud_panels_stay_on_screen_and_move() {
        let mut registry = registry();
        let info = info();
        let paints = paint_hud(&registry, &info, 640.0, 360.0);
        assert!(paints.iter().any(|p| matches!(p, Paint::Rect { .. })));
        assert!(paints.iter().any(|p| matches!(p, Paint::Text { .. })));
        for paint in &paints {
            if let Paint::Rect { x, y, w, h, .. } = paint {
                assert!(*x >= 0.0 && *y >= 0.0, "off screen: {x},{y}");
                assert!(*x + *w <= 640.5 && *y + *h <= 360.5, "past the screen");
            }
        }
        let mut menu = Menu {
            open: true,
            tab: Tab::Settings,
            subtab: SubTab::Controls,
            ..Menu::default()
        };
        menu.key(&mut registry, "key.keyboard.enter");
        assert_eq!(menu.editing, Some(0));
        assert!(menu.key(&mut registry, "key.keyboard.right"));
        assert_eq!(registry.config.anchor("fps").x, 6.0);
        assert!(menu.key(&mut registry, "key.keyboard.enter"));
        assert_eq!(registry.config.anchor("fps").corner, "top-right");
        menu.key(&mut registry, "key.keyboard.escape");
        assert!(menu.editing.is_none());
    }

    #[test]
    fn waypoints_page_enter_tracks_and_delete_removes() {
        let mut registry = registry();
        registry.config.add_waypoint(Waypoint {
            name: "A".into(),
            x: 0.0,
            y: 64.0,
            z: 0.0,
            dim: "minecraft:overworld".into(),
        });
        registry.config.add_waypoint(Waypoint {
            name: "B".into(),
            x: 4.0,
            y: 64.0,
            z: 0.0,
            dim: "minecraft:overworld".into(),
        });
        let mut menu = Menu {
            open: true,
            tab: Tab::Waypoints,
            ..Menu::default()
        };
        assert!(menu.key(&mut registry, "key.keyboard.enter"));
        assert_eq!(registry.config.tracked.as_deref(), Some("A"));
        assert!(menu.key(&mut registry, "key.keyboard.delete"));
        assert_eq!(registry.config.waypoints.len(), 1);
        assert!(registry.config.tracked.is_none());
    }

    #[test]
    fn every_tab_paints_something() {
        let registry = registry();
        for tab in Tab::ALL {
            let menu = Menu {
                tab,
                ..Menu::default()
            };
            let paints = menu.paint(&registry, 854.0, 480.0);
            assert!(paints.iter().any(|p| matches!(p, Paint::Rect { .. })));
            assert!(paints.iter().any(|p| matches!(p, Paint::Text { .. })));
        }
    }

    #[test]
    fn the_card_grid_stays_inside_the_panel() {
        let registry = registry();
        let menu = Menu::default();
        for paint in menu.paint(&registry, 640.0, 360.0) {
            if let Paint::Rect { x, y, w, h, .. } = paint {
                assert!(x >= 0.0 && y >= 0.0, "shape off the screen: {x},{y}");
                assert!(x + w <= 640.5 && y + h <= 360.5, "shape past the screen");
            }
        }
    }
}
