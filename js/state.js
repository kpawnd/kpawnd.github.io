export const STORAGE_KEY = 'kpawnd_user_files';
export const USER_INFO_KEY = 'kpawnd_user_info';

export const state = {
  system: null,
  grubMenu: null,
  grubInterval: null,
  memtest: null,
  nanoEditor: null,
  pythonRepl: false,
  terminalSetup: false,
  commandHistory: [],
  historyIndex: -1,
  user: { username: null, password: null },
  loginStage: null,
  greeted: false,
  sudoPending: null
};

export function getState() {
  return state;
}

export function setSystem(system) {
  state.system = system;
}

export function setGrubMenu(menu) {
  state.grubMenu = menu;
}

export function setGrubInterval(interval) {
  state.grubInterval = interval;
}

export function clearGrubInterval() {
  if (state.grubInterval) {
    clearInterval(state.grubInterval);
    state.grubInterval = null;
  }
}

export function setMemtest(memtest) {
  state.memtest = memtest;
}

export function setNanoEditor(editor) {
  state.nanoEditor = editor;
}

export function getNanoEditor() {
  return state.nanoEditor;
}

export function setPythonRepl(val) {
  state.pythonRepl = val;
}

export function getPythonRepl() {
  return state.pythonRepl;
}

export function setUser(user) {
  state.user = user;
}

export function getUser() {
  return state.user;
}

export function setLoginStage(stage) {
  state.loginStage = stage;
}

export function getLoginStage() {
  return state.loginStage;
}

export function setSudoPending(cmd) {
  state.sudoPending = cmd;
}

export function getSudoPending() {
  return state.sudoPending;
}
