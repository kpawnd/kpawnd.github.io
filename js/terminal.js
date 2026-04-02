import { getState, setPythonRepl, getNanoEditor, getPythonRepl, getLoginStage, setLoginStage, getUser } from './state.js';
import { print, scrollToBottom, escapeHtml, renderColorTokens } from './dom.js';
import { saveUserInfo } from './storage.js';
import { launchNanoEditor } from './nano.js';
import { showKernelPanic } from './panic.js';
import { saveUserFiles } from './storage.js';
import { doCurl, doPing, doDns, doMyIp, fetchUrl } from './network.js';

let commandHistory = [];
let historyIndex = -1;
let passwordBuffer = '';
let lastTabInput = '';
let lastTabAt = 0;

let start_doom;
let start_doom_with_difficulty;
let doom_enable_procedural;
let doom_restore_original_map;
let start_screensaver;

function setPromptText(text) {
  const promptEl = document.getElementById('prompt');
  if (!promptEl) return;
  promptEl.innerHTML = text.includes('\x1b[COLOR:') ? renderColorTokens(text) : escapeHtml(text);
}

export function initTerminal(wasm) {
  start_doom = wasm.start_doom;
  start_doom_with_difficulty = wasm.start_doom_with_difficulty || wasm.start_doom;
  start_screensaver = wasm.start_screensaver;
  doom_enable_procedural = wasm.doom_enable_procedural;
  doom_restore_original_map = wasm.doom_restore_original_map;
}

function showBootSequence(messages) {
  // Clear screen before showing boot sequence
  document.getElementById('output').innerHTML = '';

  let index = 0;
  
  function showNextMessage() {
    if (index >= messages.length) {
      // Boot complete - clear screen immediately and setup terminal
      document.getElementById('output').innerHTML = '';
      // Setup terminal like normal boot
      setPromptText(getState().system.prompt());
      return;
    }

    const message = messages[index];
    if (message && message.trim()) {
      print(message, 'boot');
      scrollToBottom();
    }
    index++;
    
    // Variable timing based on message content
    let delay = 80; // default
    if (message.includes('Loading Linux')) {
      delay = 500;
    } else if (message.includes('Starting kernel')) {
      delay = 300;
    } else if (message.includes('Loading initial ramdisk')) {
      delay = 200;
    } else if (message.includes('Command line')) {
      delay = 150;
    } else if (message.includes('Linux version')) {
      delay = 200;
    } else if (message.includes('CPU features') || message.includes('Memory:')) {
      delay = 150;
    } else if (message.includes('Loading kernel module')) {
      delay = 100;
    } else if (message.includes('Kernel initialized') || message.includes('Starting init')) {
      delay = 250;
    } else if (!message.trim()) {
      delay = 50; // empty lines faster
    }
    
    setTimeout(showNextMessage, delay);
  }

  showNextMessage();
}

export function setupTerminal() {
  const state = getState();
  if (state.terminalSetup) return;
  state.terminalSetup = true;

  const input = document.getElementById('input');
  const prompt = document.getElementById('prompt');
  const loginStage = getLoginStage();
  if (loginStage !== 'done') {
    startLogin();
  } else {
    const user = getUser();
    if (user && user.username && !state.greeted) {
      print(`Hello ${user.username}!`, 'output');
      state.greeted = true;
    }
    setPromptText(state.system.prompt());
  }

  input.addEventListener('keydown', handleTerminalKey);
  input.addEventListener('input', handleTerminalKey);
  document.addEventListener('click', () => input.focus());
  input.focus();
}

export function handleTerminalKey(e) {
  const state = getState();
  const input = document.getElementById('input');
  const loginStage = getLoginStage();
  
  // Check if we're in password mode (login password or sudo password)
  let isPasswordMode = loginStage === 'password';
  try {
    if (state.system && typeof state.system.is_waiting_for_sudo === 'function' && state.system.is_waiting_for_sudo()) {
      isPasswordMode = true;
    }
  } catch (_) {}

  // Handle password masking for input events
  if (isPasswordMode && e.type === 'input') {
    const currentValue = input.value;
    const prevLength = passwordBuffer.length;
    
    if (currentValue.length > prevLength) {
      // Characters added - handle paste or multiple chars
      const newChars = currentValue.slice(prevLength);
      // Filter out asterisks that might have been in clipboard
      const cleanChars = newChars.replace(/\*/g, '');
      passwordBuffer += cleanChars;
    } else if (currentValue.length < prevLength) {
      // Characters removed (backspace)
      passwordBuffer = passwordBuffer.slice(0, currentValue.length);
    }
    
    // Replace display with asterisks
    input.value = '*'.repeat(passwordBuffer.length);
    e.stopPropagation();
    return;
  }

  switch (e.key) {
    case 'c':
    case 'C':
      if (e.ctrlKey) {
        e.preventDefault();
        input.value = '';
        passwordBuffer = '';
        print('^C', 'info');
        if (!getPythonRepl()) {
          setPromptText(state.system.prompt());
        }
        scrollToBottom();
      }
      break;

    case 'l':
    case 'L':
      if (e.ctrlKey) {
        e.preventDefault();
        document.getElementById('output').innerHTML = '';
        if (!getPythonRepl()) {
          setPromptText(state.system.prompt());
        }
      }
      break;

    case 'Enter':
      e.preventDefault();
      let val = isPasswordMode ? passwordBuffer : input.value;
      input.value = '';
      passwordBuffer = '';
      
      // Check if backend is waiting for sudo password
      let backendWaitingSudo = false;
      try {
        backendWaitingSudo = state.system && typeof state.system.is_waiting_for_sudo === 'function' && state.system.is_waiting_for_sudo();
      } catch (_) {}
      
      if (backendWaitingSudo) {
        // Backend is waiting for sudo password, send it through normal command handler
        handleCommand(val);
        break;
      }
      
      if (loginStage && loginStage !== 'done') {
        handleLoginInput(val);
        break;
      }
      if (val.trim() && !isPasswordMode) {
        val = expandHistoryShortcut(val);
      }
      if (val.trim()) {
        if (!commandHistory.length || commandHistory[commandHistory.length - 1] !== val) {
          commandHistory.push(val);
        }
        historyIndex = commandHistory.length;
      }
      getPythonRepl() ? handlePythonInput(val) : handleCommand(val);
      input.value = '';
      break;

    case 'Tab':
      e.preventDefault();
      if (!getPythonRepl()) autocomplete(input);
      break;

    case 'ArrowUp':
      e.preventDefault();
      if (commandHistory.length && historyIndex > 0) {
        historyIndex--;
        input.value = commandHistory[historyIndex] || '';
        setTimeout(() => input.setSelectionRange(input.value.length, input.value.length), 0);
      }
      break;

    case 'ArrowDown':
      e.preventDefault();
      if (commandHistory.length) {
        if (historyIndex < commandHistory.length - 1) {
          historyIndex++;
          input.value = commandHistory[historyIndex];
        } else {
          historyIndex = commandHistory.length;
          input.value = '';
        }
        setTimeout(() => input.setSelectionRange(input.value.length, input.value.length), 0);
      }
      break;
  }
}

function startLogin() {
  setLoginStage('username');
  print('login:', 'output');
  setPromptText('');
}

function handleLoginInput(text) {
  const stage = getLoginStage();
  if (stage === 'username') {
    const username = text.trim() || 'user';
    // Store temporarily in DOM element dataset to avoid extra state complexity
    document.getElementById('input').dataset.username = username;
    print('Password:', 'output');
    setLoginStage('password');
  } else if (stage === 'password') {
    const username = document.getElementById('input').dataset.username || 'user';
    const password = text; // not validated
    saveUserInfo(username, password);
    // Inform backend of the active user so ownership and prompt reflect it
    try { getState().system.set_user(username); getState().system.set_user_password(password); } catch (e) {}
    setLoginStage('done');
    print(`Hello ${username}!`, 'output');
    // Restore normal prompt
    setPromptText(getState().system.prompt());
  }
}



export async function handleCommand(cmd) {
  const state = getState();
  const system = state.system;
  const promptText = system.prompt();
  
  if (cmd.trim() !== 'clear') {
    // Do not echo password entries when backend is waiting for sudo password
    let waiting = false;
    try {
      waiting = typeof system.is_waiting_for_sudo === 'function' && system.is_waiting_for_sudo();
    } catch (_) {
      waiting = false;
    }
    if (!waiting) {
      print(`${promptText}${cmd}`, 'command');
    }
  }

  // Delegate to backend for all commands (including sudo and reboot)

  const result = system.exec(cmd);

  // Process escape sequences
  if (result === '\x1b[CLEAR]') {
    document.getElementById('output').innerHTML = '';
  } else if (result === '\x1b[EXIT]') {
    print('logout', 'info');
  } else if (result === '\x1b[PYTHON_REPL]') {
    setPythonRepl(true);
    print('Python 3.11.0 (sandboxed, Rust-backed)', 'info');
    print('Type "exit()" to exit', 'info');
    setPromptText('>>> ');
  } else if (result.startsWith('\x1b[DOOM_ENABLE_PROC]')) {
    if (typeof doom_enable_procedural === 'function') {
      doom_enable_procedural();
      print('Procedural map enabled.', 'info');
    } else {
      print('Procedural map API not available.', 'error');
    }
  } else if (result.startsWith('\x1b[DOOM_RESTORE]')) {
    if (typeof doom_restore_original_map === 'function') {
      doom_restore_original_map();
      print('Original map restored.', 'info');
    } else {
      print('Restore map API not available.', 'error');
    }
  } else if (result === '\x1b[LAUNCH_DOOM]' || result.startsWith('\x1b[LAUNCH_DOOM:')) {
    // Handle DOOM launch
    const match = /\x1b\[LAUNCH_DOOM(?::(\d))?\]/.exec(result);
    if (match && match[1]) {
      const diff = parseInt(match[1], 10);
      start_doom_with_difficulty(diff);
    } else {
      start_doom();
    }
  } else if (result.startsWith('\x1b[LAUNCH_SNAKE]')) {
    start_doom();
  } else if (result.startsWith('\x1b[LAUNCH_SCREENSAVER]')) {
    start_screensaver();
  } else if (result.startsWith('\x1b[BOOT_SEQUENCE:')) {
    // Handle boot sequence animation
    const messagesStr = result.slice(16, -1); // Remove \x1b[BOOT_SEQUENCE: and ]
    const messages = messagesStr.split('|');
    showBootSequence(messages);
  } else if (result === '\x1b[LAUNCH_GRUB]') {
    // Show GRUB menu
    import('./grub.js').then(module => module.showGrub());
  }
  if (result.startsWith('\x1b[FETCH:')) {
    await fetchUrl(result.slice(8, -1));
  } else if (result.startsWith('\x1b[CURL:')) {
    const parts = result.slice(7, -1).split(':');
    await doCurl(parts.slice(2).join(':'), parts[0] || 'GET', parts[1] === 'true');
  } else if (result.startsWith('\x1b[PING:')) {
    await doPing(result.slice(7, -1));
  } else if (result.startsWith('\x1b[DNS:')) {
    await doDns(result.slice(6, -1));
  } else if (result.startsWith('\x1b[MYIP]')) {
    await doMyIp();
  } else if (result.startsWith('\x1b[OPEN:')) {
    window.open(result.slice(7, -1), '_blank');
  } else if (result.startsWith('\x1b[NANO:')) {
    const content = result.slice(7, -1);
    const colonIdx = content.indexOf(':');
    if (colonIdx === -1) {
      launchNanoEditor('', '');
    } else {
      launchNanoEditor(content.substring(0, colonIdx), content.substring(colonIdx + 1).replace(/\\n/g, '\n'));
    }
  } else if (result.startsWith('\x1b[KERNEL_PANIC]')) {
    showKernelPanic(result.slice(15));
  } else if (result === '\x1b[REBOOT]') {
    print('Rebooting...', 'info');
    setTimeout(() => {
      window.dispatchEvent(new CustomEvent('KP_REBOOT'));
    }, 500);
  } else if (result && result.trim()) {
    // Handle command output - clean up any remaining escape sequences
    const clean = result
      .replace(/\x1b\[COLOR:[^\]]*\]/g, '')  // \x1b[COLOR:blue], \x1b[COLOR:reset], etc.
      .replace(/\x1b\[[0-9;]*m/g, '')    // Standard ANSI escapes
      .replace(/\x1b\[[A-Z_]+[^\]]*\]/g, ''); // Other escape sequences like \x1b[SOMETHING]
    if (clean.trim()) {
      print(clean, 'output');
    }
  }

  const nanoEditor = getNanoEditor();
  // Check if backend is waiting for sudo password after this command
  let waitingSudo = false;
  try {
    waitingSudo = typeof system.is_waiting_for_sudo === 'function' && system.is_waiting_for_sudo();
  } catch (_) {}
  
  if (!getPythonRepl() && !nanoEditor && !waitingSudo) {
    setPromptText(system.prompt());
  } else if (waitingSudo) {
    // Clear prompt when waiting for sudo password (Linux-style)
    setPromptText('');
  }
  scrollToBottom();
}

function handlePythonInput(code) {
  const state = getState();
  const system = state.system;
  
  print(`>>> ${code}`, 'command');
  const result = system.exec_python(code);
  
  if (result === '\x1b[EXIT_PYTHON]') {
    setPythonRepl(false);
    setPromptText(system.prompt());
  } else if (result) {
    print(result, 'output');
  }
  scrollToBottom();
}

function autocomplete(partial) {
  const input = partial;
  const value = input.value;
  const cursor = input.selectionStart ?? value.length;
  const beforeCursor = value.slice(0, cursor);

  if (!beforeCursor || /\s$/.test(beforeCursor)) {
    return;
  }

  const state = getState();
  const system = state.system;
  const parts = beforeCursor.split(/\s+/);
  const currentToken = parts[parts.length - 1] || '';
  const isFirstToken = parts.length === 1;
  const completions = isFirstToken
    ? system.complete(currentToken)
    : system.complete_path(currentToken);

  if (!completions || completions.length === 0) {
    return;
  }

  if (completions.length === 1) {
    const replacement = completions[0];
    input.value = value.slice(0, cursor - currentToken.length) + replacement + value.slice(cursor);
    const pos = cursor - currentToken.length + replacement.length;
    input.setSelectionRange(pos, pos);
    return;
  }

  const prefix = longestCommonPrefix(completions);
  if (prefix.length > currentToken.length) {
    input.value = value.slice(0, cursor - currentToken.length) + prefix + value.slice(cursor);
    const pos = cursor - currentToken.length + prefix.length;
    input.setSelectionRange(pos, pos);
    return;
  }

  const now = Date.now();
  const isSecondTab = lastTabInput === beforeCursor && now - lastTabAt < 1200;
  lastTabInput = beforeCursor;
  lastTabAt = now;
  if (isSecondTab) {
    print(completions.join('  '), 'info');
    scrollToBottom();
  }
}

function expandHistoryShortcut(value) {
  const trimmed = value.trim();
  if (trimmed === '!!') {
    return commandHistory.length ? commandHistory[commandHistory.length - 1] : value;
  }
  if (/^!\d+$/.test(trimmed)) {
    const idx = parseInt(trimmed.slice(1), 10) - 1;
    if (idx >= 0 && idx < commandHistory.length) {
      return commandHistory[idx];
    }
  }
  return value;
}

function longestCommonPrefix(items) {
  if (!items || items.length === 0) return '';
  let prefix = items[0];
  for (let i = 1; i < items.length; i++) {
    while (!items[i].startsWith(prefix) && prefix.length > 0) {
      prefix = prefix.slice(0, -1);
    }
    if (!prefix) break;
  }
  return prefix;
}
