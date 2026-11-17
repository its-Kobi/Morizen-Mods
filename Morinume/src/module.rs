//! The one interface every Morinume module implements.
//!
//! A module owns its state and its settings; the menu only reads [`Module`]
//! and the registry calls the hooks. Adding a mod is one file plus one line
//! in [`crate::modules::all`]: no menu, card or config code per mod.

/// The chip a card answers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Hud,
    Server,
    Mechanic,
}

impl Category {
    #[allow(dead_code)] // A painted menu lists the categories; Tier 1 uses it.
    pub const ALL: [Category; 3] = [Category::Hud, Category::Server, Category::Mechanic];

    /// The section heading in SETTINGS and the chip label.
    pub fn label(self) -> &'static str {
        match self {
            Category::Hud => "HUD",
            Category::Server => "SERVER",
            Category::Mechanic => "MECHANIC",
        }
    }
}

/// What a setting row does when it is clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    /// ON/OFF, kept as a bool.
    Toggle,
    /// One of `options`, kept as an index; a click cycles to the next one.
    #[allow(dead_code)] // Tier 1's modules use choices; the tests cover them.
    Choice(&'static [&'static str]),
}

/// One row on the module's options page and in SETTINGS.
#[derive(Clone, Copy, Debug)]
pub struct Setting {
    /// Storage key, `[a-z0-9_.-]{1,64}`.
    pub key: &'static str,
    pub label: &'static str,
    pub kind: SettingKind,
    /// The value before the player changes it: `false`, or row 0.
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

    #[allow(dead_code)] // Tier 1's modules use choices; the tests cover them.
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

/// One card, its state and its hooks.
pub trait Module {
    /// Stable id: the storage key and the card's identity. Never rename it.
    fn id(&self) -> &'static str;
    /// The name on the card.
    fn name(&self) -> &'static str;
    /// The catalogue id drawn as the card's icon.
    fn icon(&self) -> &'static str {
        "minecraft:paper"
    }
    fn category(&self) -> Category;
    /// Shows the NEW chip.
    fn is_new(&self) -> bool {
        false
    }
    /// One line for the card's options page: live state, if the module has any.
    fn status(&self) -> Option<String> {
        None
    }
    /// Whether the module starts on for a profile that never touched it.
    fn enabled_by_default(&self) -> bool {
        false
    }
    fn settings(&self) -> &'static [Setting] {
        &[]
    }

    fn on_enable(&mut self) {}
    fn on_disable(&mut self) {}
    /// Every game tick (20 a second) while the module is on.
    fn on_tick(&mut self, _tick: u64) {}
    /// Every rendered frame while the module is on; keep it under 0.1 ms.
    #[allow(dead_code)] // Registration::on_frame arrives with the HUD tier.
    fn on_render(&mut self, _seconds: f32) {}
}
