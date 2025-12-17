use crate::vfs::Inode;
use crate::persist::{idb_save_vfs, idb_load_vfs};
use serde_json;

impl Inode {
    pub async fn save_to_indexeddb(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            let _ = idb_save_vfs(&json).await;
        }
    }

    pub async fn load_from_indexeddb() -> Option<Inode> {
        match idb_load_vfs().await {
            Ok(jsval) => {
                if let Some(json) = jsval.as_string() {
                    serde_json::from_str(&json).ok()
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
}
