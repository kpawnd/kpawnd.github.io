pub mod desktop;
pub mod doom;
pub mod graphics;
#[cfg(feature = "webgl")]
pub mod graphics_gl;
pub mod grub;
pub mod idle;
pub mod kernel;
pub mod memory;
pub mod nano;
pub mod neofetch;
pub mod network;
pub mod physics;
pub mod process;
pub mod python;
pub mod screensaver;
pub mod services;
pub mod shell;
pub mod system;
pub mod vfs;

pub use desktop::Desktop;
pub use doom::{memory_usage, start_doom, stop_doom};
pub use graphics::{Graphics, MatrixScreensaver, SnakeGame};
#[cfg(feature = "webgl")]
pub use graphics_gl::WebGlGraphics;
pub use grub::{GrubMenu, Memtest};
pub use idle::{set_game_active, set_screensaver_active, start_idle_timer, stop_idle_timer};
pub use nano::NanoEditor;
pub use network::{fetch_http, post_http};
pub use screensaver::{start_screensaver, stop_screensaver};
pub use system::System;

use wasm_bindgen::prelude::*;
use web_sys::window;

static INIT_HOOK: std::sync::Once = std::sync::Once::new();

fn install_panic_hook() {
    INIT_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            web_sys::console::error_1(&JsValue::from_str(&format!("PANIC: {info}")));
            // Attempt auto-restart after brief delay
            if let Some(w) = window() {
                let restart = Closure::<dyn FnMut()>::wrap(Box::new(|| {
                    // Graceful stop both subsystems then leave terminal visible
                    crate::doom::stop_doom();
                    crate::screensaver::stop_screensaver();
                }));
                let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                    restart.as_ref().unchecked_ref(),
                    250,
                );
                restart.forget();
            }
        }));
    });
}

#[wasm_bindgen]
pub fn restart_os() {
    crate::doom::stop_doom();
    crate::screensaver::stop_screensaver();
    idle::set_game_active(false);
    idle::set_screensaver_active(false);
}

#[wasm_bindgen]
pub fn init_runtime() {
    install_panic_hook();
}
