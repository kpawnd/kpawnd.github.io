use std::cell::Cell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Event};

thread_local! {
    static LAST_ACTIVITY: Cell<f64> = const { Cell::new(0.0) };
    static TIMEOUT_MS: Cell<u32> = const { Cell::new(60000) };
    static INTERVAL_HANDLE: Cell<i32> = const { Cell::new(-1) };
    static ACTIVE_GAME: Cell<bool> = const { Cell::new(false) };
    static ACTIVE_SCREENSAVER: Cell<bool> = const { Cell::new(false) };
    static CALLBACK_INSTALLED: Cell<bool> = const { Cell::new(false) };
}

pub fn mark_activity() {
    let now = js_sys::Date::now();
    LAST_ACTIVITY.with(|t| t.set(now));
}

fn attach_listeners() {
    CALLBACK_INSTALLED.with(|installed| {
        if installed.get() {
            return;
        }
        installed.set(true);
        let win = window().unwrap();
        let closure =
            wasm_bindgen::closure::Closure::<dyn FnMut(_)>::wrap(Box::new(|_e: Event| {
                mark_activity();
            }));
        for ev in [
            "mousemove",
            "keydown",
            "mousedown",
            "touchstart",
            "wheel",
            "click",
        ] {
            win.add_event_listener_with_callback(ev, closure.as_ref().unchecked_ref())
                .unwrap();
        }
        closure.forget(); // Leak to keep active for life of page
        mark_activity();
    });
}

fn launch_screensaver_if_idle() {
    ACTIVE_GAME.with(|ag| {
        ACTIVE_SCREENSAVER.with(|asv| {
            if ag.get() || asv.get() {
                return;
            }
            crate::screensaver::start_screensaver();
        });
    });
}

#[wasm_bindgen]
pub fn set_game_active(active: bool) {
    ACTIVE_GAME.with(|g| g.set(active));
    if active {
        ACTIVE_SCREENSAVER.with(|s| s.set(false));
    }
}

#[wasm_bindgen]
pub fn set_screensaver_active(active: bool) {
    ACTIVE_SCREENSAVER.with(|s| s.set(active));
}

#[wasm_bindgen]
pub fn start_idle_timer(timeout_ms: u32) {
    attach_listeners();
    TIMEOUT_MS.with(|t| t.set(timeout_ms));
    // Clear previous interval if any
    INTERVAL_HANDLE.with(|h| {
        let id = h.get();
        if id != -1 {
            window().unwrap().clear_interval_with_handle(id);
        }
    });
    let tick = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
        let now = js_sys::Date::now();
        let idle_ms = TIMEOUT_MS.with(|t| t.get()) as f64;
        let last = LAST_ACTIVITY.with(|t| t.get());
        if now - last >= idle_ms {
            launch_screensaver_if_idle();
            // Reset so we don't relaunch repeatedly
            mark_activity();
        }
    }) as Box<dyn FnMut()>);
    let id = window()
        .unwrap()
        .set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), 1000)
        .unwrap();
    INTERVAL_HANDLE.with(|h| h.set(id));
    tick.forget();
}

#[wasm_bindgen]
pub fn stop_idle_timer() {
    INTERVAL_HANDLE.with(|h| {
        let id = h.get();
        if id != -1 {
            window().unwrap().clear_interval_with_handle(id);
            h.set(-1);
        }
    });
}
