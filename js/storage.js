// File Persistence (localStorage)
import { state, STORAGE_KEY, USER_INFO_KEY, setUser, setLoginStage } from './state.js';

export function loadUserFiles() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      state.system.import_user_files(saved);
      console.log('Loaded user files from localStorage');
    }
  } catch (e) {
    console.warn('Failed to load user files:', e);
  }
}

export function saveUserFiles() {
  try {
    const files = state.system.export_user_files();
    localStorage.setItem(STORAGE_KEY, files);
    console.log('Saved user files to localStorage');
  } catch (e) {
    console.warn('Failed to save user files:', e);
  }
}

export function loadUserInfo() {
  try {
    const raw = localStorage.getItem(USER_INFO_KEY);
    if (raw) {
      const obj = JSON.parse(raw);
      if (obj && obj.username) {
        setUser({ username: obj.username, password: obj.password || null });
        setLoginStage('done');
        // Set both username AND password in backend System
        try { 
          state.system.set_user(obj.username); 
          state.system.set_user_password(obj.password || ''); 
        } catch (e) {
          console.warn('Failed to set user in system:', e);
        }
        console.log('Loaded user info');
      }
    }
  } catch (e) {
    console.warn('Failed to load user info:', e);
  }
}

export function saveUserInfo(username, password) {
  try {
    const obj = { username, password };
    localStorage.setItem(USER_INFO_KEY, JSON.stringify(obj));
    setUser(obj);
    console.log('Saved user info');
  } catch (e) {
    console.warn('Failed to save user info:', e);
  }
}
