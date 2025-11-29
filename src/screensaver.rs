use crate::graphics::Graphics;
use wasm_bindgen::prelude::*;
use web_sys::{window, Document};

fn document() -> Document {
    window().unwrap().document().unwrap()
}

type LoopClosure = std::cell::RefCell<Option<wasm_bindgen::closure::Closure<dyn FnMut(f64)>>>;

thread_local! {
    static GFX: std::cell::RefCell<Option<Graphics>> = const { std::cell::RefCell::new(None) };
    static MATRIX: std::cell::RefCell<Option<crate::graphics::MatrixScreensaver>> = const { std::cell::RefCell::new(None) };
    static LOOP: LoopClosure = const { std::cell::RefCell::new(None) };
    static KEYS: std::cell::RefCell<[bool; 256]> = const { std::cell::RefCell::new([false;256]) };
}

fn ensure_canvas(width: u32, height: u32) -> Result<web_sys::HtmlCanvasElement, JsValue> {
    use wasm_bindgen::JsCast;
    let doc = document();
    let canvas_el = doc
        .get_element_by_id("game-canvas")
        .ok_or("canvas not found")?;
    let canvas: web_sys::HtmlCanvasElement = canvas_el.dyn_into()?;
    canvas.set_width(width);
    canvas.set_height(height);
    Ok(canvas)
}

fn install_key_listeners() {
    let w = window().unwrap();
    let keydown = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::wrap(Box::new(
        |e: web_sys::KeyboardEvent| {
            KEYS.with(|k| {
                k.borrow_mut()[e.key_code() as usize] = true;
            });
        },
    ));
    let keyup = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::wrap(Box::new(
        |e: web_sys::KeyboardEvent| {
            KEYS.with(|k| {
                k.borrow_mut()[e.key_code() as usize] = false;
            });
        },
    ));
    w.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
        .unwrap();
    w.add_event_listener_with_callback("keyup", keyup.as_ref().unchecked_ref())
        .unwrap();
    keydown.forget();
    keyup.forget();
}

fn start_loop() {
    LOOP.with(|l| {
        if l.borrow().is_some() {
            return;
        }
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_ts: f64| {
            // Check for ESC key to exit - before any borrows
            let should_exit = KEYS.with(|k| k.borrow()[27]);
            if should_exit {
                stop_screensaver();
                return;
            }

            MATRIX.with(|m| {
                if let Some(ref mut saver) = *m.borrow_mut() {
                    GFX.with(|gfx| {
                        if let Some(ref mut g) = *gfx.borrow_mut() {
                            saver.update();
                            saver.render(g);
                            let _ = g.present();
                        }
                    });
                }
            });

            // Schedule next frame only if loop still exists
            LOOP.with(|l2| {
                if let Some(ref cb) = *l2.borrow() {
                    let _ = window()
                        .unwrap()
                        .request_animation_frame(cb.as_ref().unchecked_ref());
                }
            });
        }) as Box<dyn FnMut(f64)>);
        let _ = window()
            .unwrap()
            .request_animation_frame(closure.as_ref().unchecked_ref());
        *l.borrow_mut() = Some(closure);
    });
}

#[wasm_bindgen]
pub fn start_screensaver() {
    if let Some(g) = document().get_element_by_id("graphics") {
        g.set_attribute("style", "display:block;").ok();
    }
    if let Some(t) = document().get_element_by_id("terminal") {
        t.set_attribute("style", "display:none;").ok();
    }

    install_key_listeners();

    GFX.with(|gfx| {
        let w = window().unwrap();
        let width = (w.inner_width().unwrap().as_f64().unwrap() * 0.95) as u32;
        let height = (w.inner_height().unwrap().as_f64().unwrap() * 0.90) as u32;
        let _canvas = ensure_canvas(width, height).unwrap();
        let g = Graphics::new("game-canvas", width, height).unwrap();

        MATRIX.with(|m| {
            *m.borrow_mut() = Some(crate::graphics::MatrixScreensaver::new(
                g.width(),
                g.height(),
            ));
        });

        *gfx.borrow_mut() = Some(g);
    });

    crate::idle::set_game_active(false);
    crate::idle::set_screensaver_active(true);
    start_loop();
}

#[wasm_bindgen]
pub fn stop_screensaver() {
    // Stop the loop first
    LOOP.with(|l| {
        *l.borrow_mut() = None;
    });

    // Clear state
    MATRIX.with(|m| {
        *m.borrow_mut() = None;
    });
    GFX.with(|gfx| {
        *gfx.borrow_mut() = None;
    });

    // Show terminal, hide graphics
    if let Some(g) = document().get_element_by_id("graphics") {
        g.set_attribute("style", "display:none;").ok();
    }
    if let Some(t) = document().get_element_by_id("terminal") {
        t.set_attribute("style", "display:flex;").ok();
    }

    crate::idle::set_game_active(false);
    crate::idle::set_screensaver_active(false);
}
