import init, { System, GrubMenu, neofetch_logo } from './pkg/terminal_os.js';
import { Memtest } from './pkg/terminal_os.js';
let system;
let pythonRepl = null;
let grubMenu = null;
let grubInterval = null;
let terminalSetup = false;
let commandHistory = [];
let historyIndex = -1;
let memtest = null;

async function main() {
  try {
    await init();
    system = new System();
    grubMenu = new GrubMenu();
    
    showGrub();
  } catch (error) {
    document.getElementById('grub').style.display = 'none';
    document.getElementById('terminal').style.display = 'flex';
    print(`Failed to load: ${error.message}`, 'error');
  }
}

function showGrub() {
  const grubDiv = document.getElementById('grub');
  grubDiv.style.display = 'flex';
  updateGrubDisplay();
  
  // Handle keyboard input
  const handleGrubKey = (e) => {
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      grubMenu.move_up();
      updateGrubDisplay();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      grubMenu.move_down();
      updateGrubDisplay();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      bootSelected();
    }
  };
  
  document.addEventListener('keydown', handleGrubKey);
  
  // Timer countdown
  grubInterval = setInterval(() => {
    const shouldContinue = grubMenu.tick();
    updateGrubDisplay();
    
    if (!shouldContinue || grubMenu.should_boot()) {
      clearInterval(grubInterval);
      document.removeEventListener('keydown', handleGrubKey);
      bootSelected();
    }
  }, 1000);
}

function updateGrubDisplay() {
  const grubPre = document.querySelector('#grub pre');
  const rendered = grubMenu.render();
  
  // Process highlight markers
  const processed = rendered
    .replace(/\x1b\[HIGHLIGHT\]/g, '<span class="grub-selected">')
    .replace(/\x1b\[NORMAL\]/g, '</span>');
  
  grubPre.innerHTML = processed;
}

function bootSelected() {
  const selected = grubMenu.get_selected();
  
  if (selected === 1) {
    // Recovery mode - boot directly to shell without full boot log
    document.getElementById('grub').style.display = 'none';
    document.getElementById('terminal').style.display = 'flex';
    print('kpawnd v0.2.0 (recovery mode)', 'info');
    print('', 'output');
    setupTerminal();
  } else if (selected === 2) {
    // Memory test
    document.getElementById('grub').style.display = 'none';
    document.getElementById('terminal').style.display = 'flex';
    runMemtest();
  } else {
    // Normal boot
    document.getElementById('grub').style.display = 'none';
    document.getElementById('terminal').style.display = 'flex';
    beginBoot();
  }
}

function runMemtest() {
  const memSize = performance.memory ? Math.floor(performance.memory.jsHeapSizeLimit / (1024 * 1024)) : 128;
  memtest = new Memtest(memSize);
  
  const output = document.getElementById('output');
  const header = memtest.get_header();
  print(header, 'info');
  
  const testInterval = setInterval(() => {
    const shouldContinue = memtest.tick();
    
    // Clear last line
    const lines = output.children;
    if (lines.length > 3) {
      lines[lines.length - 1].remove();
    }
    
    const currentLine = memtest.get_current_line();
    print(currentLine, 'output');
    
    if (!shouldContinue || memtest.is_complete()) {
      clearInterval(testInterval);
      
      const exitHandler = (e) => {
        if (e.key === 'Escape') {
          document.removeEventListener('keydown', exitHandler);
          output.innerHTML = '';
          memtest = null;
          beginBoot();
        }
      };
      document.addEventListener('keydown', exitHandler);
    }
  }, 200);
}

function beginBoot() { system.start_boot(); drainBootLines(); }
function drainBootLines() {
  const line = system.next_boot_line();
  if (line) {
    print(line, 'boot');
    scrollToBottom();
    if (line.includes('BOOT_COMPLETE')) {
      setTimeout(() => {
        if (system.post_boot_clear_needed()) {
          system.acknowledge_post_boot();
          document.getElementById('output').innerHTML = '';
        }
        setupTerminal();
      }, 2000);
    } else {
      setTimeout(drainBootLines, 120);
    }
  }
}

function setupTerminal() {
  if (terminalSetup) return; // Prevent duplicate setup
  terminalSetup = true;
  
  const input = document.getElementById('input');
  const prompt = document.getElementById('prompt');
  prompt.textContent = system.prompt();
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      const val = input.value;
      if (val.trim() !== '') {
        // Push into history (avoid duplicate consecutive entries)
        if (commandHistory.length === 0 || commandHistory[commandHistory.length - 1] !== val) {
          commandHistory.push(val);
        }
        historyIndex = commandHistory.length; // Reset index past the end
      }
      if (pythonRepl) {
        handlePythonInput(val);
      } else {
        handleCommand(val);
      }
      input.value='';
    } else if (e.key === 'Tab') {
      e.preventDefault();
      if (!pythonRepl) {
        autocomplete(input.value);
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (commandHistory.length > 0) {
        if (historyIndex > 0) {
          historyIndex--;
        } else {
          historyIndex = 0;
        }
        input.value = commandHistory[historyIndex] || '';
        // Move cursor to end
        setTimeout(()=>input.setSelectionRange(input.value.length, input.value.length),0);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (commandHistory.length > 0) {
        if (historyIndex < commandHistory.length - 1) {
          historyIndex++;
          input.value = commandHistory[historyIndex];
        } else {
          // Past the newest entry clears input
          historyIndex = commandHistory.length;
          input.value = '';
        }
        setTimeout(()=>input.setSelectionRange(input.value.length, input.value.length),0);
      }
    }
  });
  document.addEventListener('click', () => input.focus());
  input.focus();
}

function handleCommand(cmd) {
  const promptText = system.prompt();
  if (cmd.trim() !== 'clear') {
    print(`${promptText}${cmd}`, 'command');
  }
  const result = system.exec(cmd);
  if (result === '\x1b[CLEAR]') { 
    document.getElementById('output').innerHTML = ''; 
  }
  else if (result === '\x1b[EXIT]') { print('logout', 'info'); }
  else if (result === '\x1b[NEOFETCH_DATA]') { 
    displayNeofetch();
  }
  else if (result === '\x1b[PYTHON_REPL]') { 
    pythonRepl = true;
    print('Python 3.11.0 (sandboxed, Rust-backed)', 'info');
    print('Type "exit()" to exit', 'info');
    document.getElementById('prompt').textContent = '>>> ';
  }
  else if (result.startsWith('\x1b[OPEN:')) {
    // Prefix '\x1b[OPEN:' length is 7 chars; previous 8 caused missing first character (dropping 'h' in https)
    const prefixLen = '\x1b[OPEN:'.length; // 7
    const url = result.slice(prefixLen, -1);
    window.open(url,'_blank');
  }
  else if (result) { print(result, 'output'); }
  if (!pythonRepl) {
    document.getElementById('prompt').textContent = system.prompt();
  }
  scrollToBottom();
}

function handlePythonInput(code) {
  print(`>>> ${code}`, 'command');
  const result = system.exec_python(code);
  if (result === '\x1b[EXIT_PYTHON]') {
    pythonRepl = false;
    document.getElementById('prompt').textContent = system.prompt();
  } else if (result) {
    print(result, 'output');
  }
  scrollToBottom();
}

function print(text, className='') {
  const output = document.getElementById('output');
  const line = document.createElement('div');
  line.className = `line ${className}`;
  if (text.includes('\x1b[COLOR:')) {
    line.innerHTML = renderColorTokens(text);
  } else {
    line.textContent = text;
  }
  output.appendChild(line);
}

function renderColorTokens(raw) {
  // Replace tokens \x1b[COLOR:#RRGGBB] with span wrappers.
  // Close previous span automatically when a new color token appears.
  const escapeHtml = (s) => s.replace(/[&<>]/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;'}[c]));
  const parts = [];
  let open = false;
  let lastIndex = 0;
  const regex = /\x1b\[COLOR:#([0-9A-Fa-f]{6})\]/g;
  let match;
  while ((match = regex.exec(raw)) !== null) {
    const before = raw.slice(lastIndex, match.index);
    parts.push(escapeHtml(before));
    if (open) {
      parts.push('</span>');
    }
    parts.push(`<span style="color:#${match[1]}">`);
    open = true;
    lastIndex = regex.lastIndex;
  }
  const tail = raw.slice(lastIndex);
  parts.push(escapeHtml(tail));
  if (open) {
    parts.push('</span>');
  }
  return parts.join('');
}

function autocomplete(partial) {
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

function scrollToBottom() { const term = document.getElementById('terminal'); term.scrollTop = term.scrollHeight; }

async function displayNeofetch() {
  const info = {
    os: 'Unknown', hostname: 'localhost', user: 'root', kernel: 'Unknown', browser: 'Unknown', cpu: 'Unknown cores', memory: 'Unknown', resolution: `${window.innerWidth}x${window.innerHeight}` , uptime: Math.floor(performance.now()/1000)
  };

  // Detect OS
  const ua = navigator.userAgent;
  if (ua.includes('Win')) info.os = 'Windows';
  else if (ua.includes('Mac')) info.os = 'macOS';
  else if (ua.includes('Linux')) info.os = 'Linux';
  else if (ua.includes('Android')) info.os = 'Android';
  else if (ua.includes('iOS') || ua.includes('iPhone') || ua.includes('iPad')) info.os = 'iOS';

  // Detect Browser
  if (ua.includes('Firefox')) info.browser = 'Firefox';
  else if (ua.includes('Edg')) info.browser = 'Edge';
  else if (ua.includes('Chrome')) info.browser = 'Chrome';
  else if (ua.includes('Safari') && !ua.includes('Chrome')) info.browser = 'Safari';

  // CPU - try to get more detailed info
  const cpuCores = navigator.hardwareConcurrency || 'Unknown';
  info.cpu = `${cpuCores} cores`;

  // Memory - deviceMemory is often unavailable, use rough estimate from performance
  if (navigator.deviceMemory) {
    info.memory = `${navigator.deviceMemory} GB`;
  } else if (performance.memory && performance.memory.jsHeapSizeLimit) {
    // Rough estimate: heap limit is usually 1/4 to 1/2 of available RAM
    const heapGB = (performance.memory.jsHeapSizeLimit / (1024 ** 3)).toFixed(1);
    info.memory = `~${Math.ceil(heapGB * 4)} GB (estimated)`;
  } else {
    info.memory = 'Unknown';
  }

  // Kernel/Arch
  if (ua.includes('x64') || ua.includes('x86_64') || ua.includes('Win64')) {
    info.kernel = 'x86_64';
  } else if (ua.includes('ARM') || ua.includes('aarch64')) {
    info.kernel = 'ARM64';
  } else if (ua.includes('x86')) {
    info.kernel = 'x86';
  }

  // Format uptime
  const uptimeSecs = info.uptime;
  const hours = Math.floor(uptimeSecs / 3600);
  const mins = Math.floor((uptimeSecs % 3600) / 60);
  const uptimeStr = hours > 0 ? `${hours} hours, ${mins} mins` : `${mins} mins`;

  // Get ASCII logo from Rust
  const logoBlock = neofetch_logo(info.os).split('\n');

  const infoLines = [
    `${info.user}@${info.hostname}`,
    '─'.repeat(`${info.user}@${info.hostname}`.length),
    `OS: ${info.os}`,
    `Host: ${info.browser}`,
    `Kernel: ${info.kernel}`,
    `Uptime: ${uptimeStr}`,
    `Shell: kpawnd-sh`,
    `Resolution: ${info.resolution}`,
    `Terminal: ${info.browser}`,
    `CPU: ${info.cpu}`,
  ];
  if(info.memory!=='Unknown'){ infoLines.push(`Memory: ${info.memory}`); }

  // Account for color tokens when measuring width
  const visibleWidth = (s) => s.replace(/\x1b\[COLOR:#([0-9A-Fa-f]{6})\]/g,'').length;
  const maxLogoWidth = Math.max(...logoBlock.map(l=>visibleWidth(l)));
  for(let i=0;i<Math.max(logoBlock.length, infoLines.length);i++){
    const l = logoBlock[i]||'';
    const r = infoLines[i]||'';
    const pad = ' '.repeat(maxLogoWidth - visibleWidth(l) + 3);
    print(l + pad + r,'output');
  }
  scrollToBottom();
}



main();
