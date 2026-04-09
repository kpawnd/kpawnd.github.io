import { state } from './state.js';
import { escapeHtml, scrollToBottom, getElement, renderColorTokens } from './dom.js';
import { saveUserFiles } from './storage.js';

// NanoEditor class from WASM - set by main.js
let NanoEditor;
const VISIBLE_LINES = 20;
let nanoPrompt = null;
let nanoStatus = '';
let nanoStatusTimer = null;

export function initNano(wasm) {
  NanoEditor = wasm.NanoEditor;
}

export function launchNanoEditor(filename, content) {
  state.nanoEditor = new NanoEditor(filename || 'untitled', content || '');
  getElement('prompt').style.display = 'none';
  getElement('input').style.display = 'none';
  renderNano();
  document.addEventListener('keydown', handleNanoKey);
}

export function renderNano() {
  if (!state.nanoEditor) return;

  const output = getElement('output');
  output.innerHTML = '';

  const filename = state.nanoEditor.get_filename();
  const cursorRow = state.nanoEditor.get_cursor_row();
  const cursorCol = state.nanoEditor.get_cursor_col();
  const lineCount = state.nanoEditor.line_count();
  const isModified = state.nanoEditor.is_modified();
  const startLine = state.nanoEditor.calculate_viewport_start(VISIBLE_LINES);
  const endLine = Math.min(lineCount, startLine + VISIBLE_LINES);

  // Header
  const header = document.createElement('div');
  header.className = 'line nano-header';
  Object.assign(header.style, { backgroundColor: '#fff', color: '#000', textAlign: 'center' });
  header.textContent = `  GNU nano 7.2                    ${filename}${isModified ? ' Modified' : ''}`;
  output.appendChild(header);

  // Content
  for (let i = startLine; i < endLine; i++) {
    const line = document.createElement('div');
    line.className = 'line';
    if (i === cursorRow) {
      const text = state.nanoEditor.get_line(i);
      const cursor = text[cursorCol] || ' ';
      line.innerHTML = escapeHtml(text.substring(0, cursorCol)) +
        '<span style="background:#fff;color:#000">' + escapeHtml(cursor) + '</span>' +
        escapeHtml(text.substring(cursorCol + 1));
    } else {
      line.textContent = state.nanoEditor.get_line(i);
    }
    output.appendChild(line);
  }

  // Padding
  for (let i = endLine - startLine; i < VISIBLE_LINES; i++) {
    const line = document.createElement('div');
    line.className = 'line';
    output.appendChild(line);
  }

  // Status
  const status = document.createElement('div');
  status.className = 'line';
  status.style.color = '#888';
  status.textContent = nanoStatus || `[ line ${cursorRow + 1}/${lineCount}, col ${cursorCol + 1} ]`;
  output.appendChild(status);

  if (nanoPrompt) {
    const prompt = document.createElement('div');
    prompt.className = 'line nano-prompt';
    Object.assign(prompt.style, { backgroundColor: '#222', color: '#fff' });
    prompt.textContent = `${nanoPrompt.label}${nanoPrompt.input}`;
    output.appendChild(prompt);
  }

  // Help bars
  const helpBar = (content) => {
    const el = document.createElement('div');
    el.className = 'line nano-help';
    el.style.backgroundColor = '#333';
    el.innerHTML = content;
    return el;
  };
  output.appendChild(helpBar('<span style="color:#fff">^G</span> Help    <span style="color:#fff">^O</span> Write Out  <span style="color:#fff">^W</span> Where Is  <span style="color:#fff">^K</span> Cut      <span style="color:#fff">^C</span> Location'));
  output.appendChild(helpBar('<span style="color:#fff">^X</span> Exit    <span style="color:#fff">^R</span> Read File  <span style="color:#fff">^\\</span> Replace  <span style="color:#fff">^U</span> Paste    <span style="color:#fff">^T</span> Execute'));
  scrollToBottom();
}

function handleNanoKey(e) {
  if (!state.nanoEditor) return;

  if (nanoPrompt) {
    handleNanoPromptKey(e);
    return;
  }

  const ctrlKeys = {
    x: requestExitNano,
    o: () => saveNanoFile(),
    s: () => saveNanoFile(),
    k: () => state.nanoEditor.cut_line(),
    u: () => state.nanoEditor.paste(),
    g: showNanoHelp,
    c: showCursorLocation,
    w: () => openNanoPrompt('search', 'Search: '),
    r: () => openNanoPrompt('read', 'Read file: '),
    t: () => openNanoPrompt('exec', 'Execute command: '),
    '\\': () => openNanoPrompt('replace_find', 'Replace: ')
  };

  if (e.ctrlKey && (e.key === '_' || e.key === '-' || e.key === '7')) {
    e.preventDefault();
    openNanoPrompt('goto', 'Goto line: ');
    renderNano();
    return;
  }
  if (e.ctrlKey && ctrlKeys[e.key]) {
    e.preventDefault();
    ctrlKeys[e.key]();
    if (e.key !== 'x' && e.key !== 'o') renderNano();
    return;
  }

  const navKeys = {
    ArrowUp: () => state.nanoEditor.cursor_up(),
    ArrowDown: () => state.nanoEditor.cursor_down(),
    ArrowLeft: () => state.nanoEditor.cursor_left(),
    ArrowRight: () => state.nanoEditor.cursor_right(),
    PageUp: () => state.nanoEditor.page_up(20),
    PageDown: () => state.nanoEditor.page_down(20),
    Home: () => state.nanoEditor.cursor_home(),
    End: () => state.nanoEditor.cursor_end(),
    Enter: () => state.nanoEditor.insert_newline(),
    Backspace: () => state.nanoEditor.backspace(),
    Delete: () => state.nanoEditor.delete()
  };

  if (navKeys[e.key]) {
    e.preventDefault();
    navKeys[e.key]();
    renderNano();
    return;
  }

  if (e.key.length === 1 && !e.ctrlKey && !e.altKey && !e.metaKey) {
    e.preventDefault();
    state.nanoEditor.insert_char(e.key);
    renderNano();
  }
}

function openNanoPrompt(kind, label) {
  nanoPrompt = { kind, label, input: '', data: {} };
  renderNano();
}

function clearNanoPrompt() {
  nanoPrompt = null;
}

function setNanoStatus(message, ttl = 2000) {
  nanoStatus = message;
  if (nanoStatusTimer) window.clearTimeout(nanoStatusTimer);
  nanoStatusTimer = window.setTimeout(() => {
    nanoStatus = '';
    if (state.nanoEditor) renderNano();
  }, ttl);
}

function cleanCommandOutput(result) {
  return (result || '')
    .replace(/\x1b\[COLOR:[^\]]*\]/g, '')
    .replace(/\x1b\[[0-9;]*m/g, '')
    .replace(/\x1b\[[A-Z_]+[^\]]*\]/g, '');
}

function quotePath(path) {
  if (!path) return '';
  return /\s/.test(path) ? `"${path.replace(/"/g, '\\"')}"` : path;
}

function handleNanoPromptKey(e) {
  e.preventDefault();

  if (e.key === 'Escape' || (e.ctrlKey && (e.key === 'c' || e.key === 'C'))) {
    clearNanoPrompt();
    setNanoStatus('Cancelled');
    renderNano();
    return;
  }

  if (e.key === 'Backspace') {
    nanoPrompt.input = nanoPrompt.input.slice(0, -1);
    renderNano();
    return;
  }

  if (e.key === 'Enter') {
    submitNanoPrompt();
    return;
  }

  if (e.key.length === 1 && !e.altKey && !e.metaKey && !e.ctrlKey) {
    nanoPrompt.input += e.key;
    renderNano();
  }
}

function submitNanoPrompt() {
  const value = nanoPrompt.input.trim();

  if (nanoPrompt.kind === 'search') {
    clearNanoPrompt();
    if (!value) {
      setNanoStatus('Search text required', 1600);
    } else if (state.nanoEditor.find_goto(value)) {
      setNanoStatus(`Found: ${value}`, 1400);
    } else {
      setNanoStatus(`Not found: ${value}`, 2000);
    }
    renderNano();
    return;
  }

  if (nanoPrompt.kind === 'goto') {
    clearNanoPrompt();
    const lineNum = parseInt(value, 10);
    if (Number.isFinite(lineNum) && lineNum > 0) {
      state.nanoEditor.goto_line(lineNum);
      setNanoStatus(`Moved to line ${lineNum}`, 1500);
    } else {
      setNanoStatus('Invalid line number', 1800);
    }
    renderNano();
    return;
  }

  if (nanoPrompt.kind === 'replace_find') {
    if (!value) {
      clearNanoPrompt();
      setNanoStatus('Replace text required', 1600);
      renderNano();
      return;
    }
    nanoPrompt = { kind: 'replace_with', label: 'Replace with: ', input: '', data: { needle: value } };
    renderNano();
    return;
  }

  if (nanoPrompt.kind === 'replace_with') {
    nanoPrompt = {
      kind: 'replace_scope',
      label: 'Replace all? (y/n): ',
      input: '',
      data: { needle: nanoPrompt.data.needle, replacement: value }
    };
    renderNano();
    return;
  }

  if (nanoPrompt.kind === 'replace_scope') {
    const yes = /^y(es)?$/i.test(value);
    const { needle, replacement } = nanoPrompt.data;
    clearNanoPrompt();
    if (yes) {
      const count = state.nanoEditor.replace_all(needle, replacement);
      setNanoStatus(`Replaced ${count} occurrence(s)`, 1800);
    } else {
      const changed = state.nanoEditor.replace(needle, replacement);
      setNanoStatus(changed ? 'Replaced 1 occurrence' : 'No match at cursor line', 1800);
    }
    renderNano();
    return;
  }

  if (nanoPrompt.kind === 'read') {
    clearNanoPrompt();
    if (!value) {
      setNanoStatus('File path required', 1600);
      renderNano();
      return;
    }
    const cmd = `cat ${quotePath(value)}`;
    const raw = state.system.exec(cmd);
    const out = cleanCommandOutput(raw);
    if (!out || /^cat: /.test(out)) {
      setNanoStatus(out || 'Unable to read file', 2200);
    } else {
      state.nanoEditor.insert_string(out);
      setNanoStatus(`Inserted file: ${value}`, 1700);
    }
    renderNano();
    return;
  }

  if (nanoPrompt.kind === 'exec') {
    clearNanoPrompt();
    if (!value) {
      setNanoStatus('Command required', 1600);
      renderNano();
      return;
    }
    const raw = state.system.exec(value);
    const out = cleanCommandOutput(raw).trim();
    if (out) {
      state.nanoEditor.insert_string(out);
      setNanoStatus('Command output inserted', 1800);
    } else {
      setNanoStatus('Command executed (no output)', 1500);
    }
    renderNano();
    return;
  }

  if (nanoPrompt.kind === 'exit_confirm') {
    clearNanoPrompt();
    if (/^y(es)?$/i.test(value)) {
      saveNanoFile(exitNano);
    } else if (/^n(o)?$/i.test(value)) {
      exitNano();
    } else {
      setNanoStatus('Exit cancelled', 1500);
      renderNano();
    }
  }
}

function saveNanoFile(onSuccess) {
  const content = state.nanoEditor.get_content();
  const filename = state.nanoEditor.get_filename();
  const lineCount = state.nanoEditor.line_count();
  const result = state.system.save_file(filename, content);

  if (result) {
    setNanoStatus(result, 2600);
  } else {
    state.nanoEditor.mark_saved();

    // Persist to localStorage
    saveUserFiles();

    setNanoStatus(`[ Wrote ${lineCount} lines to ${filename} ]`, 1800);
    if (typeof onSuccess === 'function') {
      window.setTimeout(onSuccess, 120);
    } else {
      renderNano();
    }
  }
}

function requestExitNano() {
  if (state.nanoEditor && state.nanoEditor.is_modified()) {
    nanoPrompt = { kind: 'exit_confirm', label: 'Save modified buffer? (y/n): ', input: '', data: {} };
    renderNano();
  } else {
    exitNano();
  }
}

function showNanoHelp() {
  setNanoStatus('^O Save  ^X Exit  ^W Search  ^\\ Replace  ^_ Goto  ^R Read  ^T Exec  ^C Cursor', 3500);
}

function showCursorLocation() {
  const row = state.nanoEditor.get_cursor_row() + 1;
  const col = state.nanoEditor.get_cursor_col() + 1;
  const total = state.nanoEditor.line_count();
  setNanoStatus(`line ${row}/${total}, col ${col}`, 1500);
}

function exitNano() {
  document.removeEventListener('keydown', handleNanoKey);
  if (state.nanoEditor) state.nanoEditor.free();
  state.nanoEditor = null;
  clearNanoPrompt();
  nanoStatus = '';
  if (nanoStatusTimer) {
    window.clearTimeout(nanoStatusTimer);
    nanoStatusTimer = null;
  }

  getElement('output').innerHTML = '';
  getElement('prompt').style.display = '';
  getElement('input').style.display = '';
  const promptText = state.system.prompt();
  const promptEl = getElement('prompt');
  promptEl.innerHTML = promptText.includes('\x1b[COLOR:') ? renderColorTokens(promptText) : escapeHtml(promptText);
  getElement('input').focus();
}
