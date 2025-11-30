import { state } from './state.js';
import { escapeHtml, scrollToBottom, getElement } from './dom.js';
import { saveUserFiles } from './storage.js';

// NanoEditor class from WASM - set by main.js
let NanoEditor;

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
  const visibleLines = 20;
  const startLine = state.nanoEditor.calculate_viewport_start(visibleLines);
  const endLine = Math.min(lineCount, startLine + visibleLines);

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
  for (let i = endLine - startLine; i < visibleLines; i++) {
    const line = document.createElement('div');
    line.className = 'line';
    output.appendChild(line);
  }

  // Status
  const status = document.createElement('div');
  status.className = 'line';
  status.style.color = '#888';
  status.textContent = `[ line ${cursorRow + 1}/${lineCount}, col ${cursorCol + 1} ]`;
  output.appendChild(status);

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

  const ctrlKeys = { x: exitNano, o: saveNanoFile, k: () => state.nanoEditor.cut_line(), u: () => state.nanoEditor.paste(), g: () => {} };
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

function saveNanoFile() {
  const content = state.nanoEditor.get_content();
  const filename = state.nanoEditor.get_filename();
  const lineCount = state.nanoEditor.line_count();
  const result = state.system.save_file(filename, content);

  if (result) {
    const output = getElement('output');
    const msg = document.createElement('div');
    msg.className = 'line';
    Object.assign(msg.style, { textAlign: 'center', color: '#f00' });
    msg.textContent = result;
    output.appendChild(msg);
  } else {
    state.nanoEditor.mark_saved();
    
    // Persist to localStorage
    saveUserFiles();
    
    const msg = document.createElement('div');
    msg.className = 'line';
    Object.assign(msg.style, { textAlign: 'center', color: '#0f0' });
    msg.textContent = `[ Wrote ${lineCount} lines to ${filename} ]`;
    getElement('output').appendChild(msg);
    setTimeout(renderNano, 1500);
  }
}

function exitNano() {
  document.removeEventListener('keydown', handleNanoKey);
  if (state.nanoEditor) state.nanoEditor.free();
  state.nanoEditor = null;

  getElement('output').innerHTML = '';
  getElement('prompt').style.display = '';
  getElement('input').style.display = '';
  getElement('prompt').textContent = state.system.prompt();
  getElement('input').focus();
}
