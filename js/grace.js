import { getState } from './state.js';
import { handleCommand as terminalHandleCommand } from './terminal.js';

(function(){
  const d = document;
  let container, desktop, panel, startMenu, windowArea;
  let windowZ = 100;
  let activeWindow = null;
  let startMenuOpen = false;

  // minimal geometry icons
  const icons = {
    start: `<svg viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" fill="#2563eb"/></svg>`,
    terminal: `<svg viewBox="0 0 24 24" fill="currentColor"><rect x="3" y="5" width="18" height="14" fill="#0f172a"/><path d="M6 9l4 3-4 3" stroke="#22c55e" stroke-width="2"/><path d="M11 15h6" stroke="#22c55e" stroke-width="2"/></svg>`,
    files: `<svg viewBox="0 0 24 24" fill="currentColor"><rect x="3" y="5" width="18" height="14" fill="#f59e0b"/><rect x="3" y="9" width="18" height="10" fill="#fbbf24"/></svg>`,
    settings: `<svg viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" fill="#9ca3af"/></svg>`,
    notepad: `<svg viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" fill="#e5e7eb"/><path d="M7 8h10M7 11h8M7 14h6M7 17h4" stroke="#6b7280" stroke-width="1.5"/></svg>`,
    info: `<svg viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" fill="#3b82f6"/><path d="M12 16v-5M12 8h1" stroke="#fff" stroke-width="2"/></svg>`,
    power: `<svg viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" fill="#ef4444"/></svg>`,
    close: `<svg viewBox="0 0 24 24"><path d="M6 6l12 12M6 18L18 6" stroke="currentColor" stroke-width="2.5"/></svg>`,
    minimize: `<svg viewBox="0 0 24 24"><path d="M5 12h14" stroke="currentColor" stroke-width="2.5"/></svg>`,
    maximize: `<svg viewBox="0 0 24 24"><rect x="6" y="6" width="12" height="12" stroke="currentColor" stroke-width="2" fill="none"/></svg>`,
  };

  function el(tag, cls, parent, html) {
    const e = d.createElement(tag);
    if (cls) e.className = cls;
    if (html) e.innerHTML = html;
    if (parent) parent.appendChild(e);
    return e;
  }

  function createDesktopIcon(name, icon, x, y, action) {
    const ic = el('div', 'grace-desktop-icon', desktop);
    ic.innerHTML = `<div class="grace-icon-img">${icon}</div><div class="grace-icon-label">${name}</div>`;
    ic.style.left = x + 'px';
    ic.style.top = y + 'px';
    ic.ondblclick = action;
  }

  function createWindow(title, content, width = 500, height = 350) {
    const win = el('div', 'grace-window', windowArea);
    win.style.width = width + 'px';
    win.style.height = height + 'px';
    win.style.left = (100 + Math.random() * 100) + 'px';
    win.style.top = (60 + Math.random() * 80) + 'px';
    win.style.zIndex = ++windowZ;

    const titlebar = el('div', 'grace-window-titlebar', win);
    const titleText = el('span', 'grace-window-title', titlebar);
    titleText.textContent = title;

    const controls = el('div', 'grace-window-controls', titlebar);
    const btnMin = el('button', 'grace-btn-minimize', controls, icons.minimize);
    const btnMax = el('button', 'grace-btn-maximize', controls, icons.maximize);
    const btnClose = el('button', 'grace-btn-close', controls, icons.close);

    btnMin.onclick = () => { win.classList.toggle('grace-minimized'); };
    btnMax.onclick = () => { win.classList.toggle('grace-maximized'); };
    btnClose.onclick = () => { win.remove(); };

    const body = el('div', 'grace-window-body', win);
    if (typeof content === 'string') {
      body.innerHTML = content;
    } else if (content instanceof HTMLElement) {
      body.appendChild(content);
    }

    // Focus on click
    win.onmousedown = () => {
      win.style.zIndex = ++windowZ;
      activeWindow = win;
    };

    // Dragging
    let dragging = false, dx = 0, dy = 0;
    titlebar.onmousedown = (e) => {
      if (e.target.closest('button')) return;
      dragging = true;
      dx = e.clientX - win.offsetLeft;
      dy = e.clientY - win.offsetTop;
      win.style.zIndex = ++windowZ;
    };
    d.addEventListener('mousemove', (e) => {
      if (!dragging) return;
      win.style.left = (e.clientX - dx) + 'px';
      win.style.top = (e.clientY - dy) + 'px';
    });
    d.addEventListener('mouseup', () => { dragging = false; });

    return { win, body };
  }

  function openFileManager() {
    const { win, body } = createWindow('Files', '<div class="grace-filemanager"></div>', 760, 460);
    const fmRoot = body.querySelector('.grace-filemanager');
    fmRoot.innerHTML = `
      <div class="grace-fm-sidebar">
        <div class="grace-fm-item" data-path="HOME">🏠 Home</div>
        <div class="grace-fm-item" data-path="/home/user/Documents">📁 Documents</div>
        <div class="grace-fm-item" data-path="/home/user/Pictures">🖼️ Pictures</div>
        <div class="grace-fm-item" data-path="/home/user/Music">🎵 Music</div>
        <div class="grace-fm-item" data-path="/home/user/Downloads">💾 Downloads</div>
      </div>
      <div class="grace-fm-content">
        <div class="grace-fm-toolbar">
          <button class="grace-fm-btn" data-act="up" title="Up one level">⬆️</button>
          <button class="grace-fm-btn" data-act="refresh" title="Refresh">🔄</button>
          <button class="grace-fm-btn" data-act="newfile" title="New File">📄+</button>
          <button class="grace-fm-btn" data-act="newfolder" title="New Folder">📁+</button>
          <button class="grace-fm-btn" data-act="delete" title="Delete Selected">🗑️</button>
          <div class="grace-fm-path" title="Current Path"></div>
        </div>
        <div class="grace-fm-main"></div>
        <div class="grace-fm-status"></div>
      </div>`;

    const sidebar = fmRoot.querySelector('.grace-fm-sidebar');
    const main = fmRoot.querySelector('.grace-fm-main');
    const toolbar = fmRoot.querySelector('.grace-fm-toolbar');
    const pathEl = fmRoot.querySelector('.grace-fm-path');
    const statusEl = fmRoot.querySelector('.grace-fm-status');

    const system = getState().system;
    let currentPath = system.exec('pwd') || '/';
    let selected = null;

    function ensureUserDirs() {
      ['Documents','Pictures','Music','Downloads'].forEach(dir => {
        system.exec(`mkdir /home/user/${dir}`); // silently ignored if exists
      });
    }

    function listDir(path) {
      const entries = system.fs_list(path);
      currentPath = path;
      pathEl.textContent = path;
      selected = null;
      main.innerHTML = '';
      let count = 0;
      if (Array.isArray(entries)) {
        entries.forEach((entry) => {
          const name = entry.name || '';
          const isDir = !!entry.is_dir;
          if (!name) return;
          const item = document.createElement('div');
          item.className = 'grace-fm-entry';
          item.dataset.name = name;
          item.dataset.isDir = isDir ? '1' : '0';
          item.innerHTML = `<div class="grace-fm-entry-icon">${isDir ? '📁' : '📄'}</div><div class="grace-fm-entry-label">${name}</div>`;
          item.onclick = () => {
            if (selected) selected.classList.remove('selected');
            selected = item;
            item.classList.add('selected');
          };
          item.ondblclick = () => {
            if (isDir) {
              const next = path === '/' ? `/${name}` : `${path}/${name}`;
              listDir(next);
            } else {
              openFileViewer(path === '/' ? `/${name}` : `${path}/${name}`);
            }
          };
          main.appendChild(item);
          count++;
        });
      }
      statusEl.textContent = `${count} items`;
    }

    function parentPath(p) {
      if (p === '/' || p === '') return '/';
      const parts = p.split('/').filter(Boolean);
      parts.pop();
      return '/' + parts.join('/');
    }

    function openFileViewer(fullPath) {
      const content = system.fs_read(fullPath);
      const viewer = createWindow(fullPath, '<pre class="grace-file-viewer"></pre>', 600, 400);
      viewer.body.querySelector('pre').textContent = content;
    }

    toolbar.addEventListener('click', (e) => {
      const btn = e.target.closest('.grace-fm-btn');
      if (!btn) return;
      const act = btn.dataset.act;
      if (act === 'up') {
        listDir(parentPath(currentPath));
      } else if (act === 'refresh') {
        listDir(currentPath);
      } else if (act === 'newfile') {
        const name = prompt('New file name');
        if (name) {
          const target = currentPath === '/' ? `/${name}` : `${currentPath}/${name}`;
          system.fs_write(target, '');
          listDir(currentPath);
        }
      } else if (act === 'newfolder') {
        const name = prompt('New folder name');
        if (name) {
          const target = currentPath === '/' ? `/${name}` : `${currentPath}/${name}`;
          system.fs_mkdir(target);
          listDir(currentPath);
        }
      } else if (act === 'delete') {
        if (!selected) return;
        const name = selected.dataset.name;
        const isDir = selected.dataset.isDir === '1';
        const target = currentPath === '/' ? `/${name}` : `${currentPath}/${name}`;
        if (confirm(`Delete ${target}?`)) {
          system.fs_rm(target, isDir);
          listDir(currentPath);
        }
      }
    });

    sidebar.addEventListener('click', (e) => {
      const item = e.target.closest('.grace-fm-item');
      if (!item) return;
      sidebar.querySelectorAll('.grace-fm-item').forEach(i => i.classList.remove('active'));
      item.classList.add('active');
      let p = item.dataset.path;
      if (p === 'HOME') {
        p = system.exec('pwd') || '/home/user';
      }
      listDir(p);
    });

    ensureUserDirs();
    listDir(currentPath);
  }

  function openTerminal() {
    // Open terminal window inside Grace instead of exiting
    openGraceTerminal();
  }

  function openGraceTerminal() {
    const { win, body } = createWindow('Terminal', '<div class="grace-terminal"></div>', 720, 480);
    const system = getState().system;
    const root = body.querySelector('.grace-terminal');
    root.innerHTML = `
      <div class="grace-term-output"></div>
      <div class="grace-term-input-row">
        <span class="grace-term-prompt"></span>
        <input class="grace-term-input" type="text" spellcheck="false" autocomplete="off" />
      </div>
    `;
    const outputEl = root.querySelector('.grace-term-output');
    const promptEl = root.querySelector('.grace-term-prompt');
    const inputEl = root.querySelector('.grace-term-input');
    const history = [];
    let histIdx = 0;

    function refreshPrompt() {
      promptEl.textContent = system.prompt();
    }

    function termPrint(text, cls = '') {
      const line = document.createElement('div');
      line.className = 'grace-term-line ' + cls;
      // Handle ANSI color codes for display
      let html = text;
      html = html.replace(/\x1b\[COLOR:([^\]]+)\]/g, '<span style="color:$1">');
      html = html.replace(/\x1b\[0m\]/g, '</span>');
      line.innerHTML = html;
      outputEl.appendChild(line);
      outputEl.scrollTop = outputEl.scrollHeight;
    }

    function termPrintHTML(html) {
      const div = document.createElement('div');
      div.className = 'grace-term-line';
      div.innerHTML = html;
      outputEl.appendChild(div);
      outputEl.scrollTop = outputEl.scrollHeight;
    }

    async function handleCmd(cmd) {
      if (cmd.trim()) {
        history.push(cmd);
        histIdx = history.length;
      }
      if (cmd.trim() !== 'clear') {
        termPrint(promptEl.textContent + cmd, 'grace-term-cmd');
      }
      const result = system.exec(cmd);
      // Handle escape sequences
      if (result === '\x1b[CLEAR]') {
        outputEl.innerHTML = '';
      } else if (result === '\x1b[EXIT]') {
        termPrint('logout', 'grace-term-info');
      } else if (result === '\x1b[NEOFETCH_DATA]') {
        // Display neofetch in terminal window
        try {
          const { neofetch_logo } = await import('../pkg/terminal_os.js');
          const logo = neofetch_logo(getState().system.exec('uname -n') || 'kpawnd');
          logo.split('\n').forEach(l => termPrintHTML('<span style="white-space:pre;font-family:monospace">' + l + '</span>'));
        } catch (e) {
          termPrint('Neofetch failed', 'grace-term-error');
        }
      } else if (result === '\x1b[PYTHON_REPL]') {
        termPrint('Python 3.11.0 (sandboxed, Rust-backed)', 'grace-term-info');
        termPrint('Type "exit()" to exit', 'grace-term-info');
      } else if (result.startsWith('\x1b[LAUNCH_DOOM')) {
        termPrint('Doom: Use fullscreen terminal for games', 'grace-term-info');
      } else if (result.startsWith('\x1b[LAUNCH_GRACE]')) {
        termPrint('Grace desktop is already running', 'grace-term-info');
      } else if (result.startsWith('\x1b[NANO:')) {
        const content = result.slice(7, -1);
        const colonIdx = content.indexOf(':');
        if (colonIdx === -1) {
          openNotepad('', '');
        } else {
          openNotepad(content.substring(0, colonIdx), content.substring(colonIdx + 1).replace(/\\n/g, '\n'));
        }
      } else if (result === '\x1b[REBOOT]') {
        termPrint('Rebooting...', 'grace-term-info');
        setTimeout(() => window.dispatchEvent(new CustomEvent('KP_REBOOT')), 500);
      } else if (result && result.trim() && !result.startsWith('\x1b[')) {
        // Strip color escapes for plain text display, then show
        const clean = result.replace(/\x1b\[COLOR:[^\]]*\]/g, '').replace(/\x1b\[0m\]/g, '');
        clean.split('\n').forEach(l => termPrint(l));
      }
      refreshPrompt();
    }

    inputEl.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        const val = inputEl.value;
        inputEl.value = '';
        handleCmd(val);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        if (histIdx > 0) { histIdx--; inputEl.value = history[histIdx] || ''; }
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        if (histIdx < history.length - 1) { histIdx++; inputEl.value = history[histIdx]; } else { histIdx = history.length; inputEl.value = ''; }
      } else if (e.key === 'Tab') {
        e.preventDefault();
        // Simple tab completion
        const partial = inputEl.value;
        if (partial && system.complete) {
          const matches = system.complete(partial);
          if (matches && matches.length === 1) {
            inputEl.value = matches[0];
          } else if (matches && matches.length > 1) {
            termPrint(matches.join('  '));
          }
        }
      }
    });

    // Focus input when clicking anywhere in terminal
    root.addEventListener('click', () => inputEl.focus());

    refreshPrompt();
    inputEl.focus();
  }

  function exitToTerminal() {
    container.style.display = 'none';
    d.dispatchEvent(new CustomEvent('GRACE:OPEN_TERMINAL'));
  }

  function openAbout() {
    const content = `
      <div style="text-align:center;padding:20px;">
        <div style="font-size:48px;margin-bottom:10px;">🌸</div>
        <h2 style="margin:0 0 10px;color:#e2e8f0;">Grace Desktop</h2>
        <p style="color:#94a3b8;margin:0;">Version 1.0</p>
        <p style="color:#64748b;margin-top:20px;font-size:13px;">A lightweight desktop environment<br>inspired by XFCE, Windows 7 & macOS</p>
      </div>`;
    createWindow('About Grace', content, 320, 260);
  }

  function openNotepad(initialPath = '', initialContent = '') {
    const { body } = createWindow('Notepad', '<div class="grace-notepad"></div>', 640, 420);
    const system = getState().system;
    const root = body.querySelector('.grace-notepad');
    root.innerHTML = `
      <div class="grace-notepad-toolbar">
        <button class="grace-notepad-btn" data-act="open">Open</button>
        <button class="grace-notepad-btn" data-act="save">Save</button>
        <button class="grace-notepad-btn" data-act="saveas">Save As</button>
        <div class="grace-notepad-path"></div>
      </div>
      <textarea class="grace-notepad-text" spellcheck="false"></textarea>
    `;
    const pathEl = root.querySelector('.grace-notepad-path');
    const textEl = root.querySelector('.grace-notepad-text');
    let currentPath = initialPath;
    if (initialContent) textEl.value = initialContent;
    pathEl.textContent = currentPath || '';

    function doOpen() {
      const p = prompt('Open file path', currentPath || getState().system.exec('pwd'));
      if (!p) return;
      const content = system.exec(`cat ${p}`);
      if (/No such file|Is a directory/.test(content)) {
        alert(`Cannot open: ${content}`);
        return;
      }
      currentPath = p;
      pathEl.textContent = p;
      textEl.value = content;
    }

    function writeFile(p, data) {
      try {
        const handle = system.sys_open(p, true);
        if (handle < 0) throw new Error('open failed');
        const ok = system.sys_write(handle >>> 0, data);
        system.sys_close(handle >>> 0);
        return ok;
      } catch (e) { return false; }
    }

    function doSave(asNew = false) {
      let p = currentPath;
      if (asNew || !p) {
        p = prompt('Save as', currentPath || (getState().system.exec('pwd') + '/untitled.txt'));
        if (!p) return;
      }
      system.exec(`touch ${p}`);
      const ok = writeFile(p, textEl.value);
      if (!ok) {
        alert('Save failed');
      } else {
        currentPath = p;
        pathEl.textContent = p;
      }
    }

    root.querySelector('.grace-notepad-toolbar').addEventListener('click', (e) => {
      const btn = e.target.closest('button');
      if (!btn) return;
      const act = btn.dataset.act;
      if (act === 'open') doOpen();
      if (act === 'save') doSave(false);
      if (act === 'saveas') doSave(true);
    });
  }

  function openSettings() {
    const content = `
      <div class="grace-settings">
        <div class="grace-settings-section">
          <h3>Appearance</h3>
          <label><input type="checkbox" checked> Enable transparency effects</label>
          <label><input type="checkbox" checked> Show desktop icons</label>
        </div>
        <div class="grace-settings-section">
          <h3>Panel</h3>
          <label>Panel position: <select><option>Bottom</option><option>Top</option></select></label>
        </div>
      </div>`;
    createWindow('Settings', content, 400, 300);
  }

  function toggleStartMenu() {
    startMenuOpen = !startMenuOpen;
    startMenu.style.display = startMenuOpen ? 'block' : 'none';
  }

  function init() {
    container = el('div', 'grace-root');
    d.body.appendChild(container);

    // Wallpaper
    el('div', 'grace-wallpaper', container);

    // Desktop area for icons
    desktop = el('div', 'grace-desktop', container);

    // Window area
    windowArea = el('div', 'grace-window-area', container);

    // Desktop icons
    createDesktopIcon('Terminal', icons.terminal, 30, 30, openTerminal);
    createDesktopIcon('Files', icons.files, 30, 120, openFileManager);
    createDesktopIcon('Notepad', icons.notepad, 30, 210, () => openNotepad());

    // Panel (taskbar)
    panel = el('div', 'grace-panel', container);

    // Start button
    const startBtn = el('button', 'grace-start-btn', panel, icons.start);
    startBtn.onclick = toggleStartMenu;

    // Quick launch
    const quickLaunch = el('div', 'grace-quick-launch', panel);
    const qlTerm = el('button', 'grace-ql-btn', quickLaunch, icons.terminal);
    qlTerm.title = 'Terminal';
    qlTerm.onclick = openTerminal;
    const qlFiles = el('button', 'grace-ql-btn', quickLaunch, icons.files);
    qlFiles.title = 'Files';
    qlFiles.onclick = openFileManager;

    // Taskbar (placeholder for open windows)
    el('div', 'grace-taskbar', panel);

    // System tray
    const tray = el('div', 'grace-tray', panel);
    const clock = el('div', 'grace-clock', tray);
    function updateClock() {
      const now = new Date();
      clock.textContent = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }
    updateClock();
    setInterval(updateClock, 1000);

    // Start menu
    startMenu = el('div', 'grace-start-menu', container);
    const menuApps = [
      { name: 'Terminal', icon: icons.terminal, action: openTerminal },
      { name: 'Files', icon: icons.files, action: openFileManager },
      { name: 'Notepad', icon: icons.notepad, action: () => openNotepad() },
      { name: 'Settings', icon: icons.settings, action: openSettings },
      { name: 'About', icon: icons.info, action: openAbout },
    ];
    menuApps.forEach(app => {
      const item = el('div', 'grace-menu-item', startMenu);
      item.innerHTML = `<span class="grace-menu-icon">${app.icon}</span><span>${app.name}</span>`;
      item.onclick = () => { toggleStartMenu(); app.action(); };
    });
    const menuDivider = el('div', 'grace-menu-divider', startMenu);
    const powerItem = el('div', 'grace-menu-item grace-menu-power', startMenu);
    powerItem.innerHTML = `<span class="grace-menu-icon">${icons.power}</span><span>Exit to Terminal</span>`;
    powerItem.onclick = () => { toggleStartMenu(); exitToTerminal(); };

    // Close menu when clicking outside
    container.onclick = (e) => {
      if (startMenuOpen && !e.target.closest('.grace-start-menu') && !e.target.closest('.grace-start-btn')) {
        toggleStartMenu();
      }
    };
  }

  // Styles moved to css/grace.css

  function launch() {
    if (!container) init();
    container.style.display = 'block';
  }

  window.GraceDesktop = { launch };
})();
