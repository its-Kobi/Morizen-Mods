//! The Tier 0 dummy module: proves the pipeline (registry -> card -> toggle
//! -> persisted -> hook) without touching gameplay, HUD or the world.

use crate::module::{Category, Module, Setting};

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

    fn icon(&self) -> &'static str {
        "minecraft:knowledge_book"
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

    fn status(&self) -> Option<String> {
        Some(format!("Ticks seen: {}", self.ticks))
    }

    fn settings(&self) -> &'static [Setting] {
        const SETTINGS: &[Setting] = &[Setting::toggle("greet", "Log when enabled", true)];
        SETTINGS
    }

    fn on_enable(&mut self) {
        self.ticks = 0;
        crate::log::info("Demo Module enabled");
    }

    fn on_disable(&mut self) {
        crate::log::info("Demo Module disabled");
    }

    fn on_tick(&mut self, _tick: u64) {
        self.ticks = self.ticks.wrapping_add(1);
    }
}
