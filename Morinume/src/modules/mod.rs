//! The module catalogue. One file per mod, one line here.
//!
//! Tier 1 (HUD) onwards adds files beside `demo.rs`; the menu, the config and
//! the card renderer never change for it.

mod demo;

pub use demo::Demo;

use crate::module::Module;

/// Every module, in card order. The menu sorts and filters this list; it
/// never reaches for a module by name.
pub fn all() -> Vec<Box<dyn Module>> {
    vec![Box::new(Demo::default())]
}
