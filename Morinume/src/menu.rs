//! The menu: state, the screen it asks the game to draw, and what a click
//! means. One card component, one widget id table; modules never touch it.

use horizon_mod_sdk::ui::{ButtonWidget, ItemWidget, ListWidget, Screen, TextWidget, Widget};

use crate::module::{Category, Module};
use crate::registry::Registry;

// Widget ids. A click comes back as `ui_select(id, row)`; row is only used
// by the list widget (profiles).
const TAB_MODS: u32 = 1;
const TAB_SETTINGS: u32 = 2;
const TAB_WAYPOINTS: u32 = 3;
const CLOSE: u32 = 4;
const SAVE_PROFILE: u32 = 5;
const EDIT_HUD: u32 = 6;
const BACK: u32 = 7;
const PROFILES: u32 = 10;
const FILTER_BASE: u32 = 20;
const SORT: u32 = 30;
const SEARCH_CLEAR: u32 = 31;
const PREV: u32 = 40;
const NEXT: u32 = 41;
const SUBTAB_BASE: u32 = 50;
/// Card icon + name (not clickable): `CARD_BASE + slot`.
const CARD_BASE: u32 = 100;
/// OPTIONS button: `OPTIONS_BASE + slot`.
const OPTIONS_BASE: u32 = 200;
/// ENABLED/DISABLED bar: `BAR_BASE + slot`.
const BAR_BASE: u32 = 300;
/// A setting row on an options or SETTINGS page: `SETTING_BASE + index`.
const SETTING_BASE: u32 = 1000;

/// The game draws at most 64 widgets per screen.
const WIDGET_CAP: usize = 64;
/// Cards per page: three columns of four, and 3 widgets each.
const CARDS_PER_PAGE: usize = 12;
/// Settings rows per page, leaving room for the header and sub-tabs.
const SETTINGS_PER_PAGE: usize = 40;

const WHITE: u32 = 0xFFFFFF;
const MUTED: u32 = 0x9AA0A6;
const BLUE: u32 = 0x5B9BD5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Mods,
    Settings,
    Waypoints,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Mods => "MODS",
            Tab::Settings => "SETTINGS",
            Tab::Waypoints => "WAYPOINTS",
        }
    }
    fn id(self) -> u32 {
        match self {
            Tab::Mods => TAB_MODS,
            Tab::Settings => TAB_SETTINGS,
            Tab::Waypoints => TAB_WAYPOINTS,
        }
    }
    fn from_id(id: u32) -> Option<Tab> {
        match id {
            TAB_MODS => Some(Tab::Mods),
            TAB_SETTINGS => Some(Tab::Settings),
            TAB_WAYPOINTS => Some(Tab::Waypoints),
            _ => None,
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
    const ALL: [Filter; 5] = [
        Filter::All,
        Filter::New,
        Filter::Hud,
        Filter::Server,
        Filter::Mechanic,
    ];

    fn label(self) -> &'static str {
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

    fn from_id(id: u32) -> Option<Filter> {
        Filter::ALL.get((id.checked_sub(FILTER_BASE)?) as usize).copied()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    /// The catalogue's order: tiers, then the order they were written in.
    #[default]
    Catalogue,
    Name,
}
/// The SETTINGS tab's sub-tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubTab {
    #[default]
    General,
    Performance,
    Controls,
}

impl SubTab {
    const ALL: [SubTab; 3] = [SubTab::General, SubTab::Performance, SubTab::Controls];

    fn label(self) -> &'static str {
        match self {
            SubTab::General => "GENERAL",
            SubTab::Performance => "PERFORMANCE",
            SubTab::Controls => "CONTROLS",
        }
    }

    fn from_id(id: u32) -> Option<SubTab> {
        SubTab::ALL.get((id.checked_sub(SUBTAB_BASE)?) as usize).copied()
    }
}

/// The menu's own state; the registry owns everything else.
#[derive(Default)]
pub struct MenuState {
    pub open: bool,
    tab: Tab,
    filter: Filter,
    sort: Sort,
    subtab: SubTab,
    search: String,
    page: usize,
    settings_page: usize,
    /// The card whose OPTIONS page shows, as a registry index.
    options_for: Option<usize>,
    /// One line under the header: the last action's result.
    notice: Option<String>,
}

impl MenuState {
    /// Opens the menu, or closes it and the screen when it is already up.
    pub fn toggle(&mut self, registry: &Registry) {
        if self.open {
            self.open = false;
            close_screen();
        } else {
            self.open = true;
            self.page = 0;
            self.options_for = None;
            self.push(registry);
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        close_screen();
    }

    /// The mod's screen is the menu (or a card's options page).
    pub fn push(&self, registry: &Registry) {
        open_screen(&self.screen(registry));
    }

    /// Rebuilds the screen after a change made elsewhere (a command).
    pub fn refresh(&self, registry: &Registry) {
        if self.open {
            self.push(registry);
        }
    }

    pub fn screen(&self, registry: &Registry) -> Screen {
        if let Some(index) = self.options_for {
            return self.options_screen(registry, index);
        }
        match self.tab {
            Tab::Mods => self.mods_screen(registry),
            Tab::Settings => self.settings_screen(registry),
            Tab::Waypoints => self.waypoints_screen(),
        }
    }

    /// The widgets Options > Mods shows on the mod's own row.
    pub fn settings_widgets(&self, registry: &Registry) -> Vec<Widget> {
        self.settings_screen(registry).widgets
    }

    /// Handles `ui_select`; true means the screen must be pushed again.
    pub fn select(&mut self, registry: &mut Registry, id: u32, row: u32) -> bool {
        if let Some(tab) = Tab::from_id(id) {
            self.tab = tab;
            self.options_for = None;
            self.page = 0;
            return true;
        }
        if id == CLOSE {
            self.open = false;
            close_screen();
            return false;
        }
        if let Some(index) = self.options_for {
            return self.select_options(registry, index, id);
        }
        match self.tab {
            Tab::Mods => self.select_mods(registry, id, row),
            Tab::Settings => self.select_settings(registry, id),
            Tab::Waypoints => true,
        }
    }

    fn select_mods(&mut self, registry: &mut Registry, id: u32, row: u32) -> bool {
        if let Some(filter) = Filter::from_id(id) {
            self.filter = filter;
            self.page = 0;
            return true;
        }
        if id == SORT {
            self.sort = match self.sort {
                Sort::Catalogue => Sort::Name,
                Sort::Name => Sort::Catalogue,
            };
            self.page = 0;
            return true;
        }
        if id == SEARCH_CLEAR {
            self.search.clear();
            self.page = 0;
            return true;
        }
        if id == PREV {
            self.page = self.page.saturating_sub(1);
            return true;
        }
        if id == NEXT {
            self.page += 1;
            self.clamp_page(registry, CARDS_PER_PAGE);
            return true;
        }
        if id == PROFILES {
            let names = registry.config().profile_names();
            if let Some(name) = names.get(row as usize) {
                registry.activate_profile(name);
                self.notice = Some(format!("Profile: {name}"));
            }
            return true;
        }
        if id == SAVE_PROFILE {
            self.notice = Some(match registry.save_as_new_profile() {
                Some(name) => format!("Saved as {name}"),
                None => "32 profiles is the limit".to_string(),
            });
            return true;
        }
        if id == EDIT_HUD {
            self.notice = Some("The HUD layout editor arrives with the HUD tier".to_string());
            return true;
        }

        let slots = self.visible(registry);
        if let Some(slot) = id.checked_sub(BAR_BASE) {
            if let Some(&index) = slots.get(slot as usize) {
                let on = registry.toggle(index);
                self.notice = Some(format!(
                    "{} {}",
                    registry.module(index).name(),
                    if on { "enabled" } else { "disabled" }
                ));
            }
            return true;
        }
        if let Some(slot) = id.checked_sub(OPTIONS_BASE) {
            if let Some(&index) = slots.get(slot as usize) {
                self.options_for = Some(index);
            }
            return true;
        }
        true
    }

    fn select_options(&mut self, registry: &mut Registry, index: usize, id: u32) -> bool {
        if id == BACK {
            self.options_for = None;
            return true;
        }
        if id == BAR_BASE {
            registry.toggle(index);
            return true;
        }
        if let Some(setting_index) = id.checked_sub(SETTING_BASE) {
            let settings = registry.module(index).settings();
            if let Some(setting) = settings.get(setting_index as usize) {
                registry.cycle_setting(index, setting);
            }
        }
        true
    }

    fn select_settings(&mut self, registry: &mut Registry, id: u32) -> bool {
        if let Some(subtab) = SubTab::from_id(id) {
            self.subtab = subtab;
            self.settings_page = 0;
            return true;
        }
        if id == SEARCH_CLEAR {
            self.search.clear();
            self.settings_page = 0;
            return true;
        }
        if id == PREV {
            self.settings_page = self.settings_page.saturating_sub(1);
            return true;
        }
        if id == NEXT {
            self.settings_page += 1;
            self.clamp_settings_page(registry);
            return true;
        }
        if let Some(setting_index) = id.checked_sub(SETTING_BASE) {
            let rows = self.setting_rows(registry);
            if let Some(&(module, setting)) = rows.get(setting_index as usize) {
                let settings = registry.module(module).settings();
                if let Some(setting) = settings.get(setting) {
                    registry.cycle_setting(module, setting);
                }
            }
        }
        true
    }

    /// Commands from chat: `/morinume ...`.
    pub fn command(
        &mut self,
        registry: &mut Registry,
        args: &str,
    ) -> Result<Option<String>, String> {
        let mut words = args.split_whitespace();
        match words.next() {
            None | Some("open" | "menu") => {
                self.open = true;
                self.options_for = None;
                self.push(registry);
                Ok(None)
            }
            Some("close") => {
                self.close();
                Ok(Some("Morinume closed".to_string()))
            }
            Some("toggle") => {
                let Some(id) = words.next() else {
                    return Err("Usage: /morinume toggle <module id>".to_string());
                };
                let index = registry
                    .index_of(id)
                    .ok_or_else(|| format!("No module `{id}`; try /morinume list"))?;
                let on = registry.toggle(index);
                self.refresh(registry);
                Ok(Some(format!(
                    "{} {}",
                    registry.module(index).name(),
                    if on { "enabled" } else { "disabled" }
                )))
            }
            Some("search") => {
                let query = words.collect::<Vec<_>>().join(" ");
                self.search = query.clone();
                self.page = 0;
                self.refresh(registry);
                Ok(Some(if query.is_empty() {
                    "Search cleared".to_string()
                } else {
                    format!("Search: {query}")
                }))
            }
            Some("profile") => match words.next() {
                None => {
                    let config = registry.config();
                    Ok(Some(format!(
                        "Profile {} (of {})",
                        config.active_profile,
                        config.profile_names().join(", ")
                    )))
                }
                Some(name) => {
                    if !registry.activate_profile(name) {
                        return Err(format!("No profile `{name}`"));
                    }
                    self.refresh(registry);
                    Ok(Some(format!("Profile: {name}")))
                }
            },
            Some("save") => Ok(Some(match registry.save_as_new_profile() {
                Some(name) => format!("Saved as {name}"),
                None => "32 profiles is the limit".to_string(),
            })),
            Some("list") => {
                let mut names = Vec::new();
                for (index, module) in registry.modules().iter().enumerate() {
                    let on = if registry.is_enabled(index) { "on" } else { "off" };
                    names.push(format!("{} ({on})", module.id()));
                }
                Ok(Some(names.join(", ")))
            }
            Some(other) => Err(format!("Unknown subcommand `{other}`")),
        }
    }

    /// The config's own settings rows (one per module setting) for SETTINGS.
    fn setting_rows(&self, registry: &Registry) -> Vec<(usize, usize)> {
        let mut rows = Vec::new();
        if self.subtab != SubTab::General {
            return rows;
        }
        for (index, module) in registry.modules().iter().enumerate() {
            if !self.matches_search(module.name()) {
                continue;
            }
            for setting_index in 0..module.settings().len() {
                rows.push((index, setting_index));
            }
        }
        rows
    }

    fn clamp_page(&mut self, registry: &Registry, per_page: usize) {
        let pages = self.visible(registry).len().div_ceil(per_page).max(1);
        self.page = self.page.min(pages - 1);
    }

    fn clamp_settings_page(&mut self, registry: &Registry) {
        let rows = self.setting_rows(registry).len();
        let pages = rows.div_ceil(SETTINGS_PER_PAGE).max(1);
        self.settings_page = self.settings_page.min(pages - 1);
    }

    fn matches_search(&self, haystack: &str) -> bool {
        let query = self.search.trim().to_lowercase();
        query.is_empty() || haystack.to_lowercase().contains(&query)
    }

    /// The registry indices on the MODS tab, filtered, searched and sorted.
    fn visible(&self, registry: &Registry) -> Vec<usize> {
        let mut indices: Vec<usize> = registry
            .modules()
            .iter()
            .enumerate()
            .filter(|(_, m)| self.filter.matches(m.as_ref()))
            .filter(|(_, m)| self.matches_search(m.name()) || self.matches_search(m.id()))
            .map(|(i, _)| i)
            .collect();
        if self.sort == Sort::Name {
            indices.sort_by_key(|&i| registry.module(i).name().to_lowercase());
        }
        indices
    }

    fn mods_screen(&self, registry: &Registry) -> Screen {
        let mut widgets = self.header(Tab::Mods);
        let config = registry.config();

        widgets.push(widget_list(
            PROFILES,
            config.profile_names(),
            config.profile_names().iter().position(|n| *n == config.active_profile),
        ));
        widgets.push(button(SAVE_PROFILE, "SAVE AS NEW PROFILE"));
        widgets.push(button(EDIT_HUD, "EDIT HUD LAYOUT"));

        for filter in Filter::ALL {
            let active = filter == self.filter;
            let label = if active {
                format!("[ {} ]", filter.label())
            } else {
                filter.label().to_string()
            };
            widgets.push(button(
                FILTER_BASE + Filter::ALL.iter().position(|f| *f == filter).unwrap_or(0) as u32,
                label,
            ));
        }
        widgets.push(button(
            SORT,
            match self.sort {
                Sort::Catalogue => "SORT: CATALOGUE",
                Sort::Name => "SORT: A-Z",
            },
        ));
        widgets.push(widget_text(
            0,
            if self.search.is_empty() {
                "SEARCH: (empty)".to_string()
            } else {
                format!("SEARCH: {}", self.search)
            },
            MUTED,
        ));
        widgets.push(button(SEARCH_CLEAR, "CLEAR SEARCH"));
        if let Some(notice) = &self.notice {
            widgets.push(widget_text(0, notice.clone(), MUTED));
        }

        let visible = self.visible(registry);
        let pages = visible.len().div_ceil(CARDS_PER_PAGE).max(1);
        let page = self.page.min(pages - 1);
        if pages > 1 {
            widgets.push(widget_text(
                0,
                format!("PAGE {}/{}", page + 1, pages),
                MUTED,
            ));
            widgets.push(button(PREV, "PREVIOUS"));
            widgets.push(button(NEXT, "NEXT"));
        }

        for (slot, &index) in visible
            .iter()
            .skip(page * CARDS_PER_PAGE)
            .take(CARDS_PER_PAGE)
            .enumerate()
        {
            let module = registry.module(index);
            let on = registry.is_enabled(index);
            widgets.push(widget_item(
                CARD_BASE + slot as u32,
                module.icon(),
                if module.is_new() {
                    format!("{}  ✦", module.name())
                } else {
                    module.name().to_string()
                },
            ));
            widgets.push(button(OPTIONS_BASE + slot as u32, "OPTIONS"));
            widgets.push(button(BAR_BASE + slot as u32, if on { "ENABLED" } else { "DISABLED" }));
        }

        if visible.is_empty() {
            widgets.push(widget_text(0, "No mods match this filter.".to_string(), MUTED));
        }

        widgets.truncate(WIDGET_CAP);
        Screen {
            title: "MORINUME".to_string(),
            widgets,
        }
    }

    fn options_screen(&self, registry: &Registry, index: usize) -> Screen {
        let module = registry.module(index);
        let on = registry.is_enabled(index);
        let mut widgets = self.header(self.tab);
        widgets.push(widget_text(
            0,
            format!("{} — {}", module.name(), module.category().label()),
            MUTED,
        ));
        if let Some(status) = module.status() {
            widgets.push(widget_text(0, status, MUTED));
        }
        widgets.push(button(BAR_BASE, if on { "ENABLED" } else { "DISABLED" }));
        widgets.push(button(BACK, "BACK TO MODS"));

        for (setting_index, setting) in module.settings().iter().enumerate() {
            widgets.push(button(
                SETTING_BASE + setting_index as u32,
                registry.setting_label(index, setting),
            ));
        }
        if module.settings().is_empty() {
            widgets.push(widget_text(0, "No options.".to_string(), MUTED));
        }
        widgets.truncate(WIDGET_CAP);
        Screen {
            title: format!("MORINUME — {}", module.name()),
            widgets,
        }
    }

    fn settings_screen(&self, registry: &Registry) -> Screen {
        let mut widgets = self.header(Tab::Settings);
        for subtab in SubTab::ALL {
            let active = subtab == self.subtab;
            widgets.push(button(
                SUBTAB_BASE + SubTab::ALL.iter().position(|s| *s == subtab).unwrap_or(0) as u32,
                if active {
                    format!("[ {} ]", subtab.label())
                } else {
                    subtab.label().to_string()
                },
            ));
        }
        widgets.push(widget_text(
            0,
            if self.search.is_empty() {
                "SEARCH: (empty)".to_string()
            } else {
                format!("SEARCH: {}", self.search)
            },
            MUTED,
        ));
        widgets.push(button(SEARCH_CLEAR, "CLEAR SEARCH"));

        let rows = self.setting_rows(registry);
        let pages = rows.len().div_ceil(SETTINGS_PER_PAGE).max(1);
        let page = self.settings_page.min(pages - 1);
        if pages > 1 {
            widgets.push(widget_text(0, format!("PAGE {}/{}", page + 1, pages), MUTED));
            widgets.push(button(PREV, "PREVIOUS"));
            widgets.push(button(NEXT, "NEXT"));
        }

        let mut last_category: Option<Category> = None;
        for (row, &(module_index, setting_index)) in rows
            .iter()
            .skip(page * SETTINGS_PER_PAGE)
            .take(SETTINGS_PER_PAGE)
            .enumerate()
        {
            let module = registry.module(module_index);
            if last_category != Some(module.category()) {
                last_category = Some(module.category());
                widgets.push(widget_text(0, module.category().label().to_string(), BLUE));
            }
            let setting = &module.settings()[setting_index];
            widgets.push(button(
                SETTING_BASE + (page * SETTINGS_PER_PAGE + row) as u32,
                format!(
                    "{} — {}",
                    module.name(),
                    registry.setting_label(module_index, setting)
                ),
            ));
        }
        if self.subtab != SubTab::General {
            widgets.push(widget_text(
                0,
                format!("Nothing under {} yet.", self.subtab.label()),
                MUTED,
            ));
        } else if rows.is_empty() {
            widgets.push(widget_text(0, "No settings yet.".to_string(), MUTED));
        }

        widgets.truncate(WIDGET_CAP);
        Screen {
            title: "MORINUME — SETTINGS".to_string(),
            widgets,
        }
    }

    fn waypoints_screen(&self) -> Screen {
        let mut widgets = self.header(Tab::Waypoints);
        widgets.push(widget_text(0, "No waypoints yet.".to_string(), MUTED));
        widgets.push(widget_text(
            0,
            "Waypoints arrive in a later tier.".to_string(),
            MUTED,
        ));
        Screen {
            title: "MORINUME — WAYPOINTS".to_string(),
            widgets,
        }
    }

    fn header(&self, active: Tab) -> Vec<Widget> {
        let mut widgets = vec![widget_text(0, "MORINUME".to_string(), WHITE)];
        for tab in [Tab::Mods, Tab::Settings, Tab::Waypoints] {
            widgets.push(button(
                tab.id(),
                if tab == active {
                    format!("[ {} ]", tab.label())
                } else {
                    tab.label().to_string()
                },
            ));
        }
        widgets.push(button(CLOSE, "X"));
        widgets
    }
}

/// Host builds (`cargo test`, clippy) have no game to draw in.
fn open_screen(screen: &Screen) {
    #[cfg(target_arch = "wasm32")]
    horizon_mod_sdk::ui::open_screen(screen);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = screen;
}

fn close_screen() {
    #[cfg(target_arch = "wasm32")]
    horizon_mod_sdk::ui::close_screen();
}

fn widget_text(id: u32, text: String, color: u32) -> Widget {    Widget::Text(TextWidget { id, text, color })
}

fn button(id: u32, label: impl Into<String>) -> Widget {
    Widget::Button(ButtonWidget {
        id,
        label: label.into(),
    })
}

fn widget_item(id: u32, item: &str, name: String) -> Widget {
    Widget::Item(ItemWidget {
        id,
        item: item.to_string(),
        name,
    })
}

fn widget_list(id: u32, rows: Vec<String>, selected: Option<usize>) -> Widget {
    Widget::List(ListWidget {
        id,
        rows,
        selected: selected.map(|s| s as u32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DEFAULT_PROFILE};
    use crate::module::{Setting, SettingKind};
    use crate::modules;

    fn registry() -> Registry {
        Registry::with_config(modules::all(), Config::default())
    }

    #[test]
    fn one_card_three_widgets_and_the_bar_toggles() {
        let mut registry = registry();
        let mut menu = MenuState::default();
        let screen = menu.screen(&registry);
        let buttons: Vec<u32> = screen
            .widgets
            .iter()
            .filter_map(|w| match w {
                Widget::Button(b) => Some(b.id),
                _ => None,
            })
            .collect();
        assert!(buttons.contains(&BAR_BASE));
        assert!(buttons.contains(&OPTIONS_BASE));
        assert!(registry.is_enabled(0));
        menu.select(&mut registry, BAR_BASE, 0);
        assert!(!registry.is_enabled(0));
    }

    #[test]
    fn filters_and_search_narrow_the_cards() {
        let registry = registry();
        let mut menu = MenuState::default();
        assert_eq!(menu.visible(&registry), vec![0]);

        menu.filter = Filter::Hud;
        assert!(menu.visible(&registry).is_empty());
        menu.filter = Filter::New;
        assert_eq!(menu.visible(&registry), vec![0]);
        menu.filter = Filter::Mechanic;
        assert_eq!(menu.visible(&registry), vec![0]);

        menu.filter = Filter::All;
        menu.search = "zzz".to_string();
        assert!(menu.visible(&registry).is_empty());
        menu.search = "demo".to_string();
        assert_eq!(menu.visible(&registry), vec![0]);
    }

    #[test]
    fn the_options_page_lists_the_modules_settings() {
        let registry = registry();
        let menu = MenuState::default();
        let options = menu.options_screen(&registry, 0);
        assert!(options.title.contains("Demo Module"));
        assert!(options.widgets.iter().any(|w| matches!(
            w,
            Widget::Button(b) if b.label == "Log when enabled: ON"
        )));
    }

    #[test]
    fn the_default_profile_is_listed() {
        let registry = registry();
        let menu = MenuState::default();
        let screen = menu.screen(&registry);
        let profiles = screen
            .widgets
            .iter()
            .find_map(|w| match w {
                Widget::List(l) => Some(l),
                _ => None,
            })
            .unwrap();
        assert_eq!(profiles.rows, vec![DEFAULT_PROFILE.to_string()]);
    }

    #[test]
    fn commands_toggle_and_search() {
        let mut registry = registry();
        let mut menu = MenuState::default();
        menu.command(&mut registry, "toggle demo").unwrap();
        assert!(!registry.is_enabled(0));
        menu.command(&mut registry, "search demo").unwrap();
        assert_eq!(menu.search, "demo");
        assert!(menu.command(&mut registry, "toggle nope").is_err());
    }

    #[test]
    fn no_screen_goes_over_the_widget_cap() {
        for tab in [Tab::Mods, Tab::Settings, Tab::Waypoints] {
            let registry = registry();
            let menu = MenuState {
                tab,
                ..MenuState::default()
            };
            assert!(menu.screen(&registry).widgets.len() <= WIDGET_CAP);
        }
    }

    #[test]
    fn setting_kinds_render_their_value() {
        let registry = registry();
        let setting = Setting::choice("unit", "Unit", &["C", "F"]);
        let label = registry.setting_label(0, &setting);
        assert_eq!(label, "Unit: C");
        assert!(matches!(setting.kind, SettingKind::Choice(_)));
    }
}
