# Morizen-Mods

The Morizen team's official mods. Each mod is a Horizon WebAssembly mod
(`docs/mods.md` upstream): one `.hmod` for every platform.

## Morinume

Morizen's mods menu: one grid of cards for every mod, its options and its
on/off switch, per profile. Right Shift or `/morinume` opens it.

**Tier 0 (this build):** the menu screen, the module registry and card
component, toggles and settings saved as JSON in the mod's `storage`, profiles,
HUD anchors, and a demo module that proves the pipeline.

| Piece | File |
|---|---|
| Module interface (id, name, category, settings, hooks) | `Morinume/src/module.rs` |
| JSON config: profiles, toggles, settings, HUD anchors | `Morinume/src/config.rs` |
| Registry: enabled state, hooks, saving | `Morinume/src/registry.rs` |
| Menu: tabs, filters, sort, search, cards, options page | `Morinume/src/menu.rs` |
| The catalogue (one line per mod) | `Morinume/src/modules/mod.rs` |

### Build

Builds run in GitHub Actions (`.github/workflows/build.yml`); nothing needs to
be installed locally except for development:

```sh
cd Morinume
cargo test                 # the pure logic, on the host
cargo horizon-mod build    # Morinume/target/horizon-mod/morinume.hmod
```

`cargo horizon-mod` comes from the game repository:
`cargo install --git https://github.com/its-Kobi/Morizen cargo-horizon-mod`.

### Install

Drop `morinume.hmod` in the game's `horizon-mods/` folder (or open it through
Options > Mods). It needs no other mod, and stays off until the player turns it
on.

### Commands

| Command | Does |
|---|---|
| `/morinume` | opens the menu |
| `/morinume close` | closes it |
| `/morinume toggle <id>` | turns a module on or off; `/morinume list` shows the ids |
| `/morinume search <text>` | filters the cards |
| `/morinume profile [name]` | lists or switches profiles |
| `/morinume save` | saves the current profile as a new one |

### The menu's look in API 0.1

The game's `ui` capability draws the menus' own widgets stacked down the middle
of a mod-owned screen: text, buttons, item icons and lists. Morinume therefore
draws each card as an item icon + name, an OPTIONS button and the
ENABLED/DISABLED bar, inside a screen the game paints. The painted dark
three-column grid of the design (custom colours, rounded cards, arbitrary
positions) needs a game-side overlay screen; the menu model, registry and
config here are UI-agnostic and ready for it (`menu::screen` is the only place
that builds widgets).

The keybind can be changed from Options > Mods > Morinume; the default is
Right Shift.
