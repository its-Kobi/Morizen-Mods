// Morinume's menu: config, module registry, menu state and the paint list.
//
// This file is both a standalone Cargo library (`cargo test` here) and the
// module `crate::game::morinume` injected into the game by
// `horizon-mixin.toml`. It touches no game type on purpose: the game glue in
// `hooks.rs` loads/saves the config and turns `Paint`s into HUD shapes.
// horizon-mixins does not accept inner file attributes, so this header uses
// ordinary comments.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The mod's id: its Options > Mods row and its settings folder.
pub const ID: &str = "morinume";
pub const VERSION: &str = "0.1.0";
/// The key name on the mod's settings page.
pub const KEY_MENU: &str = "menu";
/// The menu's key until the player binds another one.
pub const DEFAULT_MENU_KEY: &str = "key.keyboard.right.shift";
pub const DEFAULT_PROFILE: &str = "Default";

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
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Config {
            version: 1,
            enabled: BTreeMap::new(),
            settings: BTreeMap::new(),
            profiles: BTreeMap::new(),
            active_profile: DEFAULT_PROFILE.to_string(),
            hud: BTreeMap::new(),
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
        self.profiles
            .entry(self.active_profile.clone())
            .or_default();
    }

    pub fn is_enabled(&self, id: &str, default: bool) -> bool {
        let profile = self.profiles.get(&self.active_profile);
        profile
            .and_then(|p| p.get(id).copied())
            .or_else(|| self.enabled.get(id).copied())
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
        if self.profiles.len() >= 32 {
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

    pub fn activate(&mut self, name: &str) -> bool {
        if self.profiles.contains_key(name) {
            self.active_profile = name.to_string();
            true
        } else {
            false
        }
    }

    pub fn anchor(&self, id: &str) -> Anchor {
        self.hud.get(id).cloned().unwrap_or_default()
    }
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
    /// One of `options`, kept as an index. Tier 1 uses it.
    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

/// One card, its state and its hooks. The menu only reads this.
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
}

/// The Tier 0 dummy module: proves card -> toggle -> hook -> saved -> options.
#[derive(Default)]
pub struct Demo {
    ticks: u64,
}

impl Module for Demo {
    fn id(&self) -> &'static str {
        "demo"
    }
    fn name(&self) -> &'static str {
        "Demo Module"
    }
    fn accent(&self) -> u32 {
        0xE0B45A
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
        const SETTINGS: &[Setting] = &[Setting::toggle("greet", "Log when enabled", true)];
        SETTINGS
    }
    fn on_enable(&mut self) {
        self.ticks = 0;
    }
    fn on_tick(&mut self, _tick: u64) {
        self.ticks = self.ticks.wrapping_add(1);
    }
    fn status(&self) -> Option<String> {
        Some(format!("{} ticks", self.ticks))
    }
}

impl Demo {
    pub fn ticks(&self) -> u64 {
        self.ticks
    }
}

/// Every module, in card order. One line per mod; the menu never changes.
pub fn modules() -> Vec<Box<dyn Module>> {
    vec![Box::new(Demo::default())]
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
        for i in 0..registry.modules.len() {
            if registry.enabled(i) {
                registry.modules[i].on_enable();
            }
        }
        registry
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
                if self.setting_on(i, setting) { "ON" } else { "OFF" }
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
}

impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Tab::Mods => "MODS",
            Tab::Settings => "SETTINGS",
            Tab::Waypoints => "WAYPOINTS",
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
    Performance,
    Controls,
}

impl SubTab {
    pub const ALL: [SubTab; 3] = [SubTab::General, SubTab::Performance, SubTab::Controls];

    pub fn label(self) -> &'static str {
        match self {
            SubTab::General => "GENERAL",
            SubTab::Performance => "PERFORMANCE",
            SubTab::Controls => "CONTROLS",
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
    /// The highlighted row: a card, a setting row or an options row.
    pub selected: usize,
    /// The card whose OPTIONS page shows, as a registry index.
    pub options: Option<usize>,
    pub notice: Option<String>,
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
        }
    }

    /// What the player clicked (nothing yet) or pressed; true means the config
    /// changed and must be saved.
    pub fn key(&mut self, registry: &mut Registry, key: &str) -> bool {
        if !self.open {
            return false;
        }
        let mut changed = false;
        match key {
            "key.keyboard.escape" => {
                if self.options.is_some() {
                    self.options = None;
                } else {
                    self.open = false;
                }
            }
            "key.keyboard.tab" => {
                self.tab = match self.tab {
                    Tab::Mods => Tab::Settings,
                    Tab::Settings => Tab::Waypoints,
                    Tab::Waypoints => Tab::Mods,
                };
                self.selected = 0;
                self.options = None;
            }
            "key.keyboard.up" => self.selected = self.selected.saturating_sub(self.step_up()),
            "key.keyboard.down" => {
                self.selected = (self.selected + self.step_up()).min(self.rows(registry).saturating_sub(1));
            }
            "key.keyboard.left" => self.selected = self.selected.saturating_sub(1),
            "key.keyboard.right" => {
                self.selected = (self.selected + 1).min(self.rows(registry).saturating_sub(1));
            }
            "key.keyboard.enter" | "key.keyboard.space" => changed = self.activate(registry),
            "key.keyboard.o" if self.tab == Tab::Mods && self.options.is_none() => {
                let visible = self.visible(registry);
                if let Some(&index) = visible.get(self.selected) {
                    self.options = Some(index);
                    self.selected = 0;
                }
            }
            _ => {}
        }
        changed
    }

    fn step_up(&self) -> usize {
        match self.tab {
            Tab::Mods if self.options.is_none() => COLS,
            _ => 1,
        }
    }

    /// Enter on the highlighted row.
    fn activate(&mut self, registry: &mut Registry) -> bool {
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
                let rows = self.setting_rows(registry);
                if let Some(&(module, setting)) = rows.get(self.selected) {
                    let s = registry.modules[module].settings()[setting];
                    registry.cycle_setting(module, &s);
                    return true;
                }
                false
            }
            Tab::Waypoints => false,
        }
    }

    /// The rows on the current screen, for Up/Down clamping.
    fn rows(&self, registry: &Registry) -> usize {
        match (self.tab, self.options) {
            (Tab::Mods, Some(index)) => registry.modules[index].settings().len(),
            (Tab::Mods, None) => self.visible(registry).len(),
            (Tab::Settings, _) => self.setting_rows(registry).len(),
            (Tab::Waypoints, _) => 0,
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
    pub fn command(&mut self, registry: &mut Registry, args: &str) -> Result<Option<String>, String> {
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
            Some("profile") => match words.next() {
                None => Ok(Some(format!(
                    "Profile {} (of {})",
                    registry.config.active_profile,
                    registry.config.profile_names().join(", ")
                ))),
                Some(name) => {
                    if !registry.config.activate(name) {
                        return Err(format!("No profile `{name}`"));
                    }
                    Ok(Some(format!("Profile: {name}")))
                }
            },
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

const PAD: f32 = 10.0;
const TOP: f32 = 30.0;
const BAR_H: f32 = 14.0;

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
            (Tab::Waypoints, None) => self.waypoints_page(&mut p, px, py, pw, ph),
        }
        p.out
    }

    fn top_bar(&self, p: &mut Painter, registry: &Registry, px: f32, py: f32, pw: f32) {
        p.rect(px, py, pw, TOP, 0xFF12151B);
        p.rect(px + PAD, py + 8.0, 12.0, 12.0, GOLD);
        p.text("MORINUME", px + PAD + 18.0, py + 10.0, WHITE);
        let mut x = px + pw / 2.0 - 108.0;
        for tab in [Tab::Mods, Tab::Settings, Tab::Waypoints] {
            let active = tab == self.tab;
            p.text(
                tab.label(),
                x,
                py + 10.0,
                if active { BLUE } else { MUTED },
            );
            if active {
                p.rect(x, py + TOP - 3.0, 40.0, 2.0, BLUE);
            }
            x += 74.0;
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
        p.text("PROFILES", px + PAD, py + TOP + 24.0, MUTED);
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
        p.text("EDIT HUD", px + PAD + 4.0, py + ph - 27.0, MUTED);

        // Filter chips + search + sort.
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
                format!("{}  {}", m.name(), registry.setting_label(module, &m.settings()[setting])),
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
            p.text(
                if self.subtab == SubTab::General {
                    "No settings yet."
                } else {
                    "Nothing here yet."
                },
                px + PAD,
                py + TOP + 40.0,
                MUTED,
            );
        }
        self.footer(p, px, py, ph, pw);
    }

    // The card's page needs one more argument than the others (its index).
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

    fn waypoints_page(&self, p: &mut Painter, px: f32, py: f32, pw: f32, ph: f32) {
        p.text("No waypoints yet.", px + PAD, py + TOP + 24.0, MUTED);
        p.text("Waypoints arrive in a later tier.", px + PAD, py + TOP + 38.0, DIM);
        self.footer(p, px, py, ph, pw);
    }

    fn footer(&self, p: &mut Painter, px: f32, py: f32, ph: f32, pw: f32) {
        p.rect(px, py + ph - 14.0, pw, 1.0, LINE);
        p.text(
            "ARROWS MOVE   ENTER SELECT   O OPTIONS   TAB TAB   ESC CLOSE",
            px + PAD,
            py + ph - 11.0,
            DIM,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry::new(Config::default())
    }

    #[test]
    fn the_demo_is_the_first_card_and_starts_on() {
        let registry = registry();
        assert_eq!(registry.modules[0].id(), "demo");
        assert!(registry.enabled(0));
        assert_eq!(registry.modules[0].name(), "Demo Module");
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
                .get("demo"),
            Some(&false)
        );
        assert!(registry.toggle(0));
    }

    #[test]
    fn config_survives_json_round_trips() {
        let mut config = Config::default();
        config.set_enabled("demo", false);
        config.set_setting("demo", "greet", "off");
        config.hud.insert("fps".into(), Anchor { corner: "bottom-right".into(), x: 2.0, y: 30.0 });
        let text = config.to_json();
        let back = Config::from_json(&text);
        assert_eq!(config, back);
        assert!(!back.is_enabled("demo", true));
        assert_eq!(back.setting("demo", "greet"), Some("off"));
        assert_eq!(back.anchor("fps").corner, "bottom-right");
    }

    #[test]
    fn a_broken_config_falls_back_to_defaults() {
        let config = Config::from_json("not json");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn profiles_start_as_a_copy_and_have_unique_names() {
        let mut config = Config::default();
        config.set_enabled("demo", false);
        assert_eq!(config.save_as_new().as_deref(), Some("Profile 2"));
        assert!(!config.is_enabled("demo", true));
        config.set_enabled("demo", true);
        assert_eq!(config.save_as_new().as_deref(), Some("Profile 3"));
        assert!(config.activate(DEFAULT_PROFILE));
        assert!(!config.is_enabled("demo", true));
        assert!(!config.activate("Nope"));
    }

    #[test]
    fn filters_and_search_narrow_the_cards() {
        let registry = registry();
        let mut menu = Menu::default();
        assert_eq!(menu.visible(&registry), vec![0]);
        menu.filter = Filter::Hud;
        assert!(menu.visible(&registry).is_empty());
        menu.filter = Filter::New;
        assert_eq!(menu.visible(&registry), vec![0]);
        menu.filter = Filter::All;
        menu.search = "zzz".into();
        assert!(menu.visible(&registry).is_empty());
        menu.search = "demo".into();
        assert_eq!(menu.visible(&registry), vec![0]);
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
        assert_eq!(registry.config.setting("demo", "greet"), Some("off"));
        menu.key(&mut registry, "key.keyboard.escape");
        assert_eq!(menu.options, None);
        assert!(menu.open);
    }

    #[test]
    fn tab_cycles_the_three_tabs() {
        let mut registry = registry();
        let mut menu = Menu {
            open: true,
            ..Menu::default()
        };
        menu.key(&mut registry, "key.keyboard.tab");
        assert_eq!(menu.tab, Tab::Settings);
        menu.key(&mut registry, "key.keyboard.tab");
        assert_eq!(menu.tab, Tab::Waypoints);
        menu.key(&mut registry, "key.keyboard.tab");
        assert_eq!(menu.tab, Tab::Mods);
    }

    #[test]
    fn commands_toggle_search_and_list() {
        let mut registry = registry();
        let mut menu = Menu::default();
        menu.command(&mut registry, "toggle demo").unwrap();
        assert!(!registry.enabled(0));
        menu.command(&mut registry, "search demo").unwrap();
        assert_eq!(menu.search, "demo");
        assert!(menu.command(&mut registry, "toggle nope").is_err());
        let list = menu.command(&mut registry, "list").unwrap().unwrap();
        assert!(list.contains("demo (off)"));
    }

    #[test]
    fn every_tab_paints_something() {
        let registry = registry();
        for tab in [Tab::Mods, Tab::Settings, Tab::Waypoints] {
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
