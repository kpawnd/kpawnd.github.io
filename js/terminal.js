import { getState, setSystem, setPythonRepl, setNanoEditor, getNanoEditor, getPythonRepl, getLoginStage, setLoginStage, getUser, setSudoPending, getSudoPending } from './state.js';
import { print, scrollToBottom, escapeHtml } from './dom.js';
import { saveUserInfo } from './storage.js';
import { displayNeofetch } from './neofetch.js';
import { launchNanoEditor } from './nano.js';
import { showKernelPanic } from './panic.js';
import { saveUserFiles } from './storage.js';
import { doCurl, doPing, doDns, doMyIp, fetchUrl } from './network.js';

let commandHistory = [];
let historyIndex = -1;
let passwordBuffer = '';

let start_doom;
let start_screensaver;

export function initTerminal(wasm) {
  start_doom = wasm.start_doom;
  start_screensaver = wasm.start_screensaver;
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
    prompt.textContent = state.system.prompt();
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
    case 'Enter':
      e.preventDefault();
      const val = isPasswordMode ? passwordBuffer : input.value;
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
      if (!getPythonRepl()) autocomplete(input.value);
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
  document.getElementById('prompt').textContent = '';
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
    document.getElementById('prompt').textContent = getState().system.prompt();
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
  } else if (result === '\x1b[NEOFETCH_DATA]') {
    displayNeofetch();
  } else if (result === '\x1b[PYTHON_REPL]') {
    setPythonRepl(true);
    print('Python 3.11.0 (sandboxed, Rust-backed)', 'info');
    print('Type "exit()" to exit', 'info');
    document.getElementById('prompt').textContent = '>>> ';
  } else if (result.startsWith('\x1b[LAUNCH_DOOM]') || result.startsWith('\x1b[LAUNCH_SNAKE]')) {
    start_doom();
  } else if (result.startsWith('\x1b[LAUNCH_SCREENSAVER]')) {
    start_screensaver();
  } else if (result.startsWith('\x1b[FETCH:')) {
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
    // If simulated shell doesn't recognize the command, try OS fallback
    if (result.startsWith('sh:') && result.endsWith('command not found')) {
      const trimmedCmd = cmd.trim();
      // Do NOT fallback for sudo/reboot — keep it in simulated OS
      if (trimmedCmd.startsWith('sudo ') || trimmedCmd === 'reboot') {
        print(result, 'error');
        return;
      }
      try {
        const resp = await fetch('/exec', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ cmd })
        });
        const data = await resp.json();
        const out = `${data.stdout || ''}${data.stderr || ''}`.trim();
        print(out || '(no output)', 'output');
      } catch (e) {
        print(`exec error: ${e.message}`, 'error');
      }
    } else {
      print(result, 'output');
    }
  }

  const nanoEditor = getNanoEditor();
  // Check if backend is waiting for sudo password after this command
  let waitingSudo = false;
  try {
    waitingSudo = typeof system.is_waiting_for_sudo === 'function' && system.is_waiting_for_sudo();
  } catch (_) {}
  
  if (!getPythonRepl() && !nanoEditor && !waitingSudo) {
    document.getElementById('prompt').textContent = system.prompt();
  } else if (waitingSudo) {
    // Clear prompt when waiting for sudo password (Linux-style)
    document.getElementById('prompt').textContent = '';
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
    document.getElementById('prompt').textContent = system.prompt();
  } else if (result) {
    print(result, 'output');
  }
  scrollToBottom();
}

function autocomplete(partial) {
  const state = getState();
  const system = state.system;
  const parts = partial.split(' ');
  const word = parts[parts.length - 1];
  const completions = system.complete(word);

  if (completions.length === 1) {
    parts[parts.length - 1] = completions[0];
    document.getElementById('input').value = parts.join(' ');
  } else if (completions.length > 1) {
    print(completions.join('  '), 'info');
  }
}
