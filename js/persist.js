// IndexedDB persistence for VFS
const DB_NAME = 'kpawnd-vfs';
const STORE_NAME = 'vfs';

export async function idb_save_vfs(data) {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 1);
    req.onupgradeneeded = () => {
      req.result.createObjectStore(STORE_NAME);
    };
    req.onsuccess = () => {
      const db = req.result;
      const tx = db.transaction(STORE_NAME, 'readwrite');
      tx.objectStore(STORE_NAME).put(data, 'root');
      tx.oncomplete = () => { db.close(); resolve(); };
      tx.onerror = (e) => { db.close(); reject(e); };
    };
    req.onerror = reject;
  });
}

export async function idb_load_vfs() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 1);
    req.onupgradeneeded = () => {
      req.result.createObjectStore(STORE_NAME);
    };
    req.onsuccess = () => {
      const db = req.result;
      const tx = db.transaction(STORE_NAME, 'readonly');
      const getReq = tx.objectStore(STORE_NAME).get('root');
      getReq.onsuccess = () => { db.close(); resolve(getReq.result || null); };
      getReq.onerror = (e) => { db.close(); reject(e); };
    };
    req.onerror = reject;
  });
}
