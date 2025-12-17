use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(module = "/js/persist.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    pub async fn idb_save_vfs(data: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(catch)]
    pub async fn idb_load_vfs() -> Result<JsValue, JsValue>;
}
