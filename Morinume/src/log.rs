//! The mod's log, silent on host builds (`cargo test`, clippy) where there is
//! no game to write into.

pub fn info(message: &str) {
    #[cfg(target_arch = "wasm32")]
    horizon_mod_sdk::info(message);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = message;
}

pub fn warn(message: &str) {
    #[cfg(target_arch = "wasm32")]
    horizon_mod_sdk::warn(message);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = message;
}
