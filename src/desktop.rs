//! Grace Desktop Environment - System 6/7 Style
//!
//! A lightweight desktop environment inspired by classic Macintosh System 6/7.
//! Renders to HTML/CSS via wasm-bindgen.

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

/// Window state for the desktop
#[derive(Clone)]
struct DesktopWindow {
    id: u32,
    title: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    // minimized: bool, // Removed to fix clippy dead_code warning
    window_type: WindowType,
}

#[derive(Clone, PartialEq)]
enum WindowType {
    Terminal,
    FileManager,
    Notepad,
    About,
}

thread_local! {
    static DESKTOP_STATE: RefCell<DesktopState> = const { RefCell::new(DesktopState::new_const()) };
}

struct DesktopState {
    windows: Vec<DesktopWindow>,
    next_window_id: u32,
    active_window_id: Option<u32>,
    z_index: u32,
    visible: bool,
    current_path: String,
    terminal_history: Vec<String>,
    terminal_history_idx: usize,
}

impl DesktopState {
    const fn new_const() -> Self {
        Self {
            windows: Vec::new(),
            next_window_id: 1,
            active_window_id: None,
            z_index: 100,
            visible: false,
            current_path: String::new(),
            terminal_history: Vec::new(),
            terminal_history_idx: 0,
        }
    }
}

/// Desktop Environment manager
#[wasm_bindgen]
pub struct Desktop;

#[wasm_bindgen]
impl Desktop {
    /// Launch the desktop environment
    #[wasm_bindgen]
    pub fn launch() {
        DESKTOP_STATE.with(|state| {
            let mut s = state.borrow_mut();
            s.visible = true;
            s.current_path = "/home/user".to_string();
        });
        Self::render_desktop();
    }

    /// Hide the desktop and return to terminal
    #[wasm_bindgen]
    pub fn hide() {
        DESKTOP_STATE.with(|state| {
            state.borrow_mut().visible = false;
        });
        if let Some(root) = Self::get_root() {
            root.set_inner_html("");
            let _ = root.set_attribute("style", "display:none");
        }
    }

    /// Check if desktop is visible
    #[wasm_bindgen]
    pub fn is_visible() -> bool {
        DESKTOP_STATE.with(|state| state.borrow().visible)
    }

    /// Open a terminal window
    #[wasm_bindgen]
    pub fn open_terminal() {
        Self::create_window("Terminal", WindowType::Terminal, 600, 400);
    }

    /// Open the file manager
    #[wasm_bindgen]
    pub fn open_files() {
        Self::create_window("Files", WindowType::FileManager, 500, 350);
    }

    /// Open notepad
    #[wasm_bindgen]
    pub fn open_notepad() {
        Self::create_window("Notepad", WindowType::Notepad, 480, 360);
    }

    /// Open about dialog
    #[wasm_bindgen]
    pub fn open_about() {
        Self::create_window("About This Computer", WindowType::About, 300, 200);
    }

    fn get_document() -> Option<Document> {
        web_sys::window()?.document()
    }

    fn get_root() -> Option<Element> {
        Self::get_document()?.query_selector(".grace-root").ok()?
    }

    fn create_window(title: &str, window_type: WindowType, width: u32, height: u32) {
        let window_id = DESKTOP_STATE.with(|state| {
            let mut s = state.borrow_mut();
            let id = s.next_window_id;
            s.next_window_id += 1;
            s.z_index += 1;
            let win = DesktopWindow {
                id,
                title: title.to_string(),
                x: 50 + (id as i32 * 20) % 100,
                y: 40 + (id as i32 * 20) % 80,
                width,
                height,
                window_type,
            };
            s.windows.push(win);
            s.active_window_id = Some(id);
            id
        });
        Self::render_window(window_id);
    }

    fn render_desktop() {
        let doc = match Self::get_document() {
            Some(d) => d,
            None => return,
        };

        // Create or get root element
        let root = match doc.query_selector(".grace-root").ok().flatten() {
            Some(r) => r,
            None => {
                let r = doc.create_element("div").unwrap();
                r.set_class_name("grace-root");
                doc.body().unwrap().append_child(&r).unwrap();
                r
            }
        };

        let _ = root.set_attribute("style", "display:block");

        // Use r##" "## to allow # inside the string
        let hd_icon = r##"<svg viewBox="0 0 32 32" fill="none" stroke="#000" stroke-width="1.5"><rect x="4" y="6" width="24" height="20" rx="1"></rect><path d="M4 10h24"></path><rect x="8" y="3" width="10" height="7" rx="1"></rect></svg>"##;
        let trash_icon = r##"<svg viewBox="0 0 32 32" fill="none" stroke="#000" stroke-width="1.5"><path d="M8 10h16v18H8z"></path><path d="M6 10h20"></path><path d="M12 6h8v4h-8z"></path><path d="M12 14v10M16 14v10M20 14v10"></path></svg>"##;

        let html = [
            r#"<div class="s7-desktop">"#,
            r#"<div class="s7-menubar">"#,
            r##"<div class="s7-apple-menu" onclick="window.GraceDesktop.toggleAppleMenu()">&#xF8FF;</div>"##,
            r#"<div class="s7-menu-title">File</div>"#,
            r#"<div class="s7-menu-title">Edit</div>"#,
            r#"<div class="s7-menu-title">View</div>"#,
            r#"<div class="s7-menu-title">Special</div>"#,
            r#"<div class="s7-menu-spacer"></div>"#,
            r#"<div class="s7-menu-clock" id="s7-clock"></div>"#,
            r#"</div>"#,
            r#"<div class="s7-desktop-area" id="s7-desktop-area">"#,
            r#"<div class="s7-icon" ondblclick="window.GraceDesktop.openFiles()">"#,
            r#"<div class="s7-icon-img">"#,
            hd_icon,
            r#"</div>"#,
            r#"<div class="s7-icon-label">HD</div>"#,
            r#"</div>"#,
            r#"<div class="s7-icon" style="top:90px" ondblclick="window.GraceDesktop.openTrash()">"#,
            r#"<div class="s7-icon-img">"#,
            trash_icon,
            r#"</div>"#,
            r#"<div class="s7-icon-label">Trash</div>"#,
            r#"</div>"#,
            r#"</div>"#,
            r#"<div class="s7-apple-dropdown" id="s7-apple-dropdown" style="display:none">"#,
            r#"<div class="s7-dropdown-item" onclick="window.GraceDesktop.openAbout()">About This Computer...</div>"#,
            r#"<div class="s7-dropdown-sep"></div>"#,
            r#"<div class="s7-dropdown-item" onclick="window.GraceDesktop.openTerminal()">Terminal</div>"#,
            r#"<div class="s7-dropdown-item" onclick="window.GraceDesktop.openNotepad()">Notepad</div>"#,
            r#"<div class="s7-dropdown-item" onclick="window.GraceDesktop.openFiles()">Files</div>"#,
            r#"<div class="s7-dropdown-sep"></div>"#,
            r#"<div class="s7-dropdown-item" onclick="window.GraceDesktop.shutdown()">Shut Down...</div>"#,
            r#"</div>"#,
            r#"<div class="s7-windows" id="s7-windows"></div>"#,
            r#"</div>"#,
        ].join("\n");

        root.set_inner_html(&html);

        // Start clock
        Self::update_clock();

        // Expose to JS
        Self::expose_to_js();
    }

    fn render_window(window_id: u32) {
        let doc = match Self::get_document() {
            Some(d) => d,
            None => return,
        };

        let win_data = DESKTOP_STATE.with(|state| {
            state
                .borrow()
                .windows
                .iter()
                .find(|w| w.id == window_id)
                .cloned()
        });

        let win = match win_data {
            Some(w) => w,
            None => return,
        };

        let z = DESKTOP_STATE.with(|state| state.borrow().z_index);

        let container = match doc.query_selector("#s7-windows").ok().flatten() {
            Some(c) => c,
            None => return,
        };

        // Create window element
        let win_el = doc.create_element("div").unwrap();
        win_el.set_class_name("s7-window");
        win_el.set_id(&format!("s7-win-{}", window_id));
        let _ = win_el.set_attribute(
            "style",
            &format!(
                "left:{}px;top:{}px;width:{}px;height:{}px;z-index:{}",
                win.x, win.y, win.width, win.height, z
            ),
        );

        // Window content based on type
        let content = match win.window_type {
            WindowType::Terminal => Self::render_terminal_content(window_id),
            WindowType::FileManager => Self::render_filemanager_content(window_id),
            WindowType::Notepad => Self::render_notepad_content(window_id),
            WindowType::About => Self::render_about_content(),
        };

        win_el.set_inner_html(&format!(r#"
            <div class="s7-titlebar" data-winid="{}" onmousedown="window.GraceDesktop.startDrag({}, event)">
                <div class="s7-close-box" onclick="event.stopPropagation(); window.GraceDesktop.closeWindow({})"></div>
                <div class="s7-title">{}</div>
                <div class="s7-zoom-box"></div>
            </div>
            <div class="s7-window-body">{}</div>
            <div class="s7-resize-handle" data-winid="{}"></div>
        "#, window_id, window_id, window_id, win.title, content, window_id));

        container.append_child(&win_el).unwrap();

        // Setup window interactions
        Self::setup_window_drag(window_id);

        // Setup terminal if it's a terminal window
        if win.window_type == WindowType::Terminal {
            Self::setup_terminal(window_id);
        } else if win.window_type == WindowType::FileManager {
            Self::setup_filemanager(window_id);
        }
    }

    fn render_terminal_content(window_id: u32) -> String {
        format!(
            r#"
            <div class="s7-terminal" id="s7-term-{}">
                <div class="s7-term-output" id="s7-term-out-{}"></div>
                <div class="s7-term-input-line">
                    <span class="s7-term-prompt" id="s7-term-prompt-{}">$ </span>
                    <input type="text" class="s7-term-input" id="s7-term-input-{}" autocomplete="off" spellcheck="false">
                </div>
            </div>
        "#,
            window_id, window_id, window_id, window_id
        )
    }

    fn render_filemanager_content(window_id: u32) -> String {
        format!(
            r#"
            <div class="s7-filemanager" id="s7-fm-{}">
                <div class="s7-fm-toolbar">
                    <button class="s7-btn" onclick="(async () => await window.GraceDesktop.fmUp({}))()">↑ Up</button>
                    <button class="s7-btn" onclick="(async () => await window.GraceDesktop.fmNewFolder({}))()">New Folder</button>
                    <button class="s7-btn" onclick="(async () => await window.GraceDesktop.fmDelete({}))()">Delete</button>
                </div>
                <div class="s7-fm-pathbar">
                    <span class="s7-fm-path" id="s7-fm-path-{}">/home/user</span>
                </div>
                <div class="s7-fm-list" id="s7-fm-list-{}"></div>
                <div class="s7-fm-status" id="s7-fm-status-{}">0 items</div>
            </div>
        "#,
            window_id, window_id, window_id, window_id, window_id, window_id, window_id
        )
    }

    fn render_notepad_content(window_id: u32) -> String {
        format!(
            r#"
            <div class="s7-notepad" id="s7-notepad-{}">
                <div class="s7-notepad-toolbar">
                    <button class="s7-btn" onclick="(async () => await window.GraceDesktop.notepadOpen({}))()">Open</button>
                    <button class="s7-btn" onclick="(async () => await window.GraceDesktop.notepadSave({}))()">Save</button>
                    <button class="s7-btn" onclick="(async () => await window.GraceDesktop.notepadSaveAs({}))()">Save As</button>
                    <span class="s7-notepad-path" id="s7-notepad-path-{}"></span>
                </div>
                <textarea class="s7-notepad-text" id="s7-notepad-text-{}" spellcheck="false"></textarea>
            </div>
        "#,
            window_id, window_id, window_id, window_id, window_id, window_id
        )
    }

    fn render_about_content() -> String {
        r#"
            <div class="s7-about">
                <div class="s7-about-icon">&#127800;</div>
                <div class="s7-about-title">Grace Desktop</div>
                <div class="s7-about-ver">Version 1.0</div>
                <div class="s7-about-text">A lightweight desktop environment<br>inspired by Macintosh System 7</div>
                <div class="s7-about-mem">Built-in Memory: 128 MB</div>
            </div>
        "#.to_string()
    }

    fn setup_window_drag(window_id: u32) {
        // Window dragging is handled by JavaScript via the GraceDesktop bridge
        // We just need to mark the window as draggable
        let doc = match Self::get_document() {
            Some(d) => d,
            None => return,
        };

        // Setup drag via inline event handlers in the HTML (already done)
        // Just make the window focusable
        if let Some(win_el) = doc
            .query_selector(&format!("#s7-win-{}", window_id))
            .ok()
            .flatten()
        {
            let _ = win_el.set_attribute("tabindex", "0");
        }
    }

    fn setup_terminal(window_id: u32) {
        let doc = match Self::get_document() {
            Some(d) => d,
            None => return,
        };

        let input_el = match doc
            .query_selector(&format!("#s7-term-input-{}", window_id))
            .ok()
            .flatten()
        {
            Some(i) => i,
            None => return,
        };

        // Set initial prompt
        if let Some(_prompt_el) = doc
            .query_selector(&format!("#s7-term-prompt-{}", window_id))
            .ok()
            .flatten()
        {
            // We'll set the prompt from JS since we need system access
        }

        let wid = window_id;
        let keydown =
            Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
                let key = e.key();
                if key == "Enter" {
                    e.prevent_default();
                    // Call into JS to handle command execution
                    if let Some(win) = web_sys::window() {
                        let _ = js_sys::Reflect::get(&win, &JsValue::from_str("GraceDesktop"))
                            .ok()
                            .and_then(|gd| {
                                js_sys::Reflect::get(
                                    &gd,
                                    &JsValue::from_str("handleTerminalCommand"),
                                )
                                .ok()
                                .and_then(|f| {
                                    f.dyn_ref::<js_sys::Function>().map(|func| {
                                        func.call1(&gd, &JsValue::from_f64(wid as f64)).ok()
                                    })
                                })
                            });
                    }
                } else if key == "ArrowUp" {
                    e.prevent_default();
                    if let Some(win) = web_sys::window() {
                        let _ = js_sys::Reflect::get(&win, &JsValue::from_str("GraceDesktop"))
                            .ok()
                            .and_then(|gd| {
                                js_sys::Reflect::get(&gd, &JsValue::from_str("terminalHistoryUp"))
                                    .ok()
                                    .and_then(|f| {
                                        f.dyn_ref::<js_sys::Function>().map(|func| {
                                            func.call1(&gd, &JsValue::from_f64(wid as f64)).ok()
                                        })
                                    })
                            });
                    }
                } else if key == "ArrowDown" {
                    e.prevent_default();
                    if let Some(win) = web_sys::window() {
                        let _ = js_sys::Reflect::get(&win, &JsValue::from_str("GraceDesktop"))
                            .ok()
                            .and_then(|gd| {
                                js_sys::Reflect::get(&gd, &JsValue::from_str("terminalHistoryDown"))
                                    .ok()
                                    .and_then(|f| {
                                        f.dyn_ref::<js_sys::Function>().map(|func| {
                                            func.call1(&gd, &JsValue::from_f64(wid as f64)).ok()
                                        })
                                    })
                            });
                    }
                }
            });

        input_el
            .add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
            .unwrap();
        keydown.forget();

        // Focus the input
        if let Some(inp) = input_el.dyn_ref::<HtmlInputElement>() {
            let _ = inp.focus();
        }
    }

    fn setup_filemanager(window_id: u32) {
        // Initial listing will be triggered from JS
        if let Some(win) = web_sys::window() {
            let _ = js_sys::Reflect::get(&win, &JsValue::from_str("GraceDesktop"))
                .ok()
                .and_then(|gd| {
                    js_sys::Reflect::get(&gd, &JsValue::from_str("refreshFileManager"))
                        .ok()
                        .and_then(|f| {
                            f.dyn_ref::<js_sys::Function>().map(|func| {
                                func.call1(&gd, &JsValue::from_f64(window_id as f64)).ok()
                            })
                        })
                });
        }
    }

    fn update_clock() {
        let update = Closure::<dyn FnMut()>::new(move || {
            if let Some(doc) = Self::get_document() {
                if let Some(clock) = doc.query_selector("#s7-clock").ok().flatten() {
                    let date = js_sys::Date::new_0();
                    let h = date.get_hours();
                    let m = date.get_minutes();
                    let ampm = if h >= 12 { "PM" } else { "AM" };
                    let h12 = if h == 0 {
                        12
                    } else if h > 12 {
                        h - 12
                    } else {
                        h
                    };
                    clock.set_inner_html(&format!("{}:{:02} {}", h12, m, ampm));
                }
            }
        });

        if let Some(win) = web_sys::window() {
            let _ = win.set_interval_with_callback_and_timeout_and_arguments_0(
                update.as_ref().unchecked_ref(),
                1000,
            );
        }
        update.forget();
    }

    fn expose_to_js() {
        // This creates the GraceDesktop global object that JS can call
        // The actual command handling is done in the JS bridge
    }

    /// Close a window
    #[wasm_bindgen]
    pub fn close_window(window_id: u32) {
        DESKTOP_STATE.with(|state| {
            state.borrow_mut().windows.retain(|w| w.id != window_id);
        });

        if let Some(doc) = Self::get_document() {
            if let Some(win) = doc
                .query_selector(&format!("#s7-win-{}", window_id))
                .ok()
                .flatten()
            {
                win.remove();
            }
        }
    }

    /// Toggle the menu dropdown
    #[wasm_bindgen]
    pub fn toggle_apple_menu() {
        if let Some(doc) = Self::get_document() {
            if let Some(dropdown) = doc.query_selector("#s7-apple-dropdown").ok().flatten() {
                let style = dropdown.dyn_ref::<HtmlElement>().map(|el| el.style());
                if let Some(style) = style {
                    let current = style.get_property_value("display").unwrap_or_default();
                    let _ = style
                        .set_property("display", if current == "none" { "block" } else { "none" });
                }
            }
        }
    }

    /// Shutdown returns to terminal
    #[wasm_bindgen]
    pub fn shutdown() {
        Self::hide();
        // Dispatch event to show terminal
        if let Some(doc) = Self::get_document() {
            let event = web_sys::CustomEvent::new("GRACE:OPEN_TERMINAL").unwrap();
            let _ = doc.dispatch_event(&event);
        }
    }

    /// Get current path for file manager
    #[wasm_bindgen]
    pub fn get_current_path() -> String {
        DESKTOP_STATE.with(|state| state.borrow().current_path.clone())
    }

    /// Set current path
    #[wasm_bindgen]
    pub fn set_current_path(path: &str) {
        DESKTOP_STATE.with(|state| {
            state.borrow_mut().current_path = path.to_string();
        });
    }

    /// Add to terminal history
    #[wasm_bindgen]
    pub fn add_terminal_history(cmd: &str) {
        DESKTOP_STATE.with(|state| {
            let mut s = state.borrow_mut();
            if !cmd.trim().is_empty() {
                s.terminal_history.push(cmd.to_string());
                s.terminal_history_idx = s.terminal_history.len();
            }
        });
    }

    /// Get history item (for up arrow)
    #[wasm_bindgen]
    pub fn get_history_prev() -> Option<String> {
        DESKTOP_STATE.with(|state| {
            let mut s = state.borrow_mut();
            if s.terminal_history_idx > 0 {
                s.terminal_history_idx -= 1;
                s.terminal_history.get(s.terminal_history_idx).cloned()
            } else {
                None
            }
        })
    }

    /// Get history item (for down arrow)
    #[wasm_bindgen]
    pub fn get_history_next() -> Option<String> {
        DESKTOP_STATE.with(|state| {
            let mut s = state.borrow_mut();
            if s.terminal_history_idx < s.terminal_history.len() {
                s.terminal_history_idx += 1;
                if s.terminal_history_idx >= s.terminal_history.len() {
                    Some(String::new())
                } else {
                    s.terminal_history.get(s.terminal_history_idx).cloned()
                }
            } else {
                Some(String::new())
            }
        })
    }

    /// Open trash
    #[wasm_bindgen]
    pub fn open_trash() {
        // Placeholder
    }
}
