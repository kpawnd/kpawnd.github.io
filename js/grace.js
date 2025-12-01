(function(){
  const d = document;
  let container, desktop, panel, startMenu, windowArea;
  let windowZ = 100;
  let activeWindow = null;
  let startMenuOpen = false;

  // SVG Icons
  const icons = {
    start: `<svg viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="12" r="10" fill="#4a90d9"/><path d="M7 7h4v4H7zM13 7h4v4h-4zM7 13h4v4H7zM13 13h4v4h-4z" fill="#fff"/></svg>`,
    terminal: `<svg viewBox="0 0 24 24" fill="currentColor"><rect x="2" y="3" width="20" height="18" rx="2" fill="#1a1a2e"/><path d="M5 7l4 4-4 4M10 15h6" stroke="#4ade80" stroke-width="2" fill="none"/></svg>`,
    files: `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M3 6a2 2 0 012-2h4l2 2h8a2 2 0 012 2v10a2 2 0 01-2 2H5a2 2 0 01-2-2V6z" fill="#f59e0b"/><path d="M3 10h18v8a2 2 0 01-2 2H5a2 2 0 01-2-2v-8z" fill="#fbbf24"/></svg>`,
    browser: `<svg viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="12" r="10" fill="#3b82f6"/><ellipse cx="12" cy="12" rx="10" ry="4" fill="none" stroke="#93c5fd" stroke-width="1.5"/><ellipse cx="12" cy="12" rx="4" ry="10" fill="none" stroke="#93c5fd" stroke-width="1.5"/><line x1="2" y1="12" x2="22" y2="12" stroke="#93c5fd" stroke-width="1.5"/></svg>`,
    settings: `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 15a3 3 0 100-6 3 3 0 000 6z" fill="#6b7280"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z" fill="#9ca3af"/></svg>`,
    notepad: `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M4 4a2 2 0 012-2h8l6 6v12a2 2 0 01-2 2H6a2 2 0 01-2-2V4z" fill="#e5e7eb"/><path d="M14 2v6h6" fill="#9ca3af"/><path d="M7 13h10M7 17h7" stroke="#6b7280" stroke-width="1.5"/></svg>`,
    info: `<svg viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="12" r="10" fill="#3b82f6"/><path d="M12 16v-4M12 8h.01" stroke="#fff" stroke-width="2" stroke-linecap="round"/></svg>`,
    power: `<svg viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="12" r="10" fill="#ef4444"/><path d="M12 6v6M8 8a6 6 0 108 0" stroke="#fff" stroke-width="2" stroke-linecap="round" fill="none"/></svg>`,
    close: `<svg viewBox="0 0 24 24"><path d="M6 6l12 12M6 18L18 6" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/></svg>`,
    minimize: `<svg viewBox="0 0 24 24"><path d="M5 12h14" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/></svg>`,
    maximize: `<svg viewBox="0 0 24 24"><rect x="5" y="5" width="14" height="14" rx="1" stroke="currentColor" stroke-width="2" fill="none"/></svg>`,
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
    const content = `
      <div class="grace-filemanager">
        <div class="grace-fm-sidebar">
          <div class="grace-fm-item active">🏠 Home</div>
          <div class="grace-fm-item">📁 Documents</div>
          <div class="grace-fm-item">🖼️ Pictures</div>
          <div class="grace-fm-item">🎵 Music</div>
          <div class="grace-fm-item">💾 Downloads</div>
        </div>
        <div class="grace-fm-main">
          <div class="grace-fm-file"><span>📄</span> readme.txt</div>
          <div class="grace-fm-file"><span>📁</span> projects</div>
          <div class="grace-fm-file"><span>🖼️</span> wallpaper.png</div>
          <div class="grace-fm-file"><span>📄</span> notes.md</div>
        </div>
      </div>`;
    createWindow('Files', content, 600, 400);
  }

  function openTerminal() {
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
    createDesktopIcon('Browser', icons.browser, 30, 210, () => createWindow('Browser', '<iframe src="about:blank" style="width:100%;height:100%;border:none;"></iframe>', 800, 500));

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
    powerItem.onclick = () => { toggleStartMenu(); openTerminal(); };

    // Close menu when clicking outside
    container.onclick = (e) => {
      if (startMenuOpen && !e.target.closest('.grace-start-menu') && !e.target.closest('.grace-start-btn')) {
        toggleStartMenu();
      }
    };
  }

  function ensureStyles() {
    if (d.getElementById('grace-styles')) return;
    const style = d.createElement('style');
    style.id = 'grace-styles';
    style.textContent = `
    /* Grace Desktop - XFCE/Win7/OSX inspired */
    .grace-root { position: fixed; inset: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; font-size: 13px; user-select: none; overflow: hidden; }
    .grace-wallpaper { position: absolute; inset: 0; background: linear-gradient(135deg, #1e3a5f 0%, #0d1b2a 50%, #1b263b 100%); }
    .grace-wallpaper::before { content: ''; position: absolute; inset: 0; background: radial-gradient(ellipse at 30% 20%, rgba(100,150,200,0.15) 0%, transparent 50%), radial-gradient(ellipse at 70% 80%, rgba(50,100,150,0.1) 0%, transparent 40%); }
    
    .grace-desktop { position: absolute; top: 0; left: 0; right: 0; bottom: 48px; }
    .grace-desktop-icon { position: absolute; width: 74px; text-align: center; cursor: pointer; padding: 8px 4px; border-radius: 6px; transition: background 0.15s; }
    .grace-desktop-icon:hover { background: rgba(255,255,255,0.1); }
    .grace-desktop-icon:active { background: rgba(255,255,255,0.2); }
    .grace-icon-img { width: 48px; height: 48px; margin: 0 auto 6px; }
    .grace-icon-img svg { width: 100%; height: 100%; filter: drop-shadow(0 2px 4px rgba(0,0,0,0.3)); }
    .grace-icon-label { color: #fff; font-size: 11px; text-shadow: 0 1px 3px rgba(0,0,0,0.8); word-wrap: break-word; }

    .grace-window-area { position: absolute; top: 0; left: 0; right: 0; bottom: 48px; pointer-events: none; }
    .grace-window { position: absolute; background: linear-gradient(180deg, #3a3a4a 0%, #2d2d3a 100%); border-radius: 8px; box-shadow: 0 8px 32px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.1); overflow: hidden; pointer-events: auto; display: flex; flex-direction: column; }
    .grace-window.grace-minimized { display: none; }
    .grace-window.grace-maximized { left: 0 !important; top: 0 !important; width: 100% !important; height: calc(100% - 48px) !important; border-radius: 0; }
    
    .grace-window-titlebar { height: 32px; background: linear-gradient(180deg, #4a4a5a 0%, #3a3a4a 100%); display: flex; align-items: center; padding: 0 8px; cursor: move; flex-shrink: 0; }
    .grace-window-title { flex: 1; color: #e2e8f0; font-weight: 500; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .grace-window-controls { display: flex; gap: 6px; }
    .grace-window-controls button { width: 14px; height: 14px; border: none; border-radius: 50%; cursor: pointer; display: flex; align-items: center; justify-content: center; padding: 0; transition: filter 0.15s; }
    .grace-window-controls button svg { width: 8px; height: 8px; }
    .grace-btn-minimize { background: #f59e0b; color: #7c4a03; }
    .grace-btn-maximize { background: #22c55e; color: #065f2c; }
    .grace-btn-close { background: #ef4444; color: #7f1d1d; }
    .grace-window-controls button:hover { filter: brightness(1.2); }
    
    .grace-window-body { flex: 1; background: #1e1e2e; color: #e2e8f0; overflow: auto; }

    .grace-panel { position: absolute; bottom: 0; left: 0; right: 0; height: 48px; background: linear-gradient(180deg, rgba(40,40,50,0.95) 0%, rgba(25,25,35,0.98) 100%); backdrop-filter: blur(12px); border-top: 1px solid rgba(255,255,255,0.1); display: flex; align-items: center; padding: 0 8px; gap: 8px; z-index: 10000; }
    
    .grace-start-btn { width: 40px; height: 40px; border: none; border-radius: 8px; background: linear-gradient(180deg, #4a90d9 0%, #2563eb 100%); cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.15s; box-shadow: 0 2px 8px rgba(37,99,235,0.3); }
    .grace-start-btn:hover { background: linear-gradient(180deg, #60a5fa 0%, #3b82f6 100%); transform: scale(1.05); }
    .grace-start-btn svg { width: 28px; height: 28px; }

    .grace-quick-launch { display: flex; gap: 4px; padding: 0 8px; border-right: 1px solid rgba(255,255,255,0.1); }
    .grace-ql-btn { width: 36px; height: 36px; border: none; border-radius: 6px; background: transparent; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: background 0.15s; }
    .grace-ql-btn:hover { background: rgba(255,255,255,0.1); }
    .grace-ql-btn svg { width: 24px; height: 24px; }

    .grace-taskbar { flex: 1; }

    .grace-tray { display: flex; align-items: center; gap: 12px; padding: 0 12px; }
    .grace-clock { color: #e2e8f0; font-size: 12px; font-weight: 500; }

    .grace-start-menu { position: absolute; bottom: 56px; left: 8px; width: 280px; background: linear-gradient(180deg, rgba(45,45,58,0.98) 0%, rgba(30,30,42,0.99) 100%); backdrop-filter: blur(16px); border-radius: 12px; box-shadow: 0 8px 32px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.1); padding: 8px 0; display: none; z-index: 10001; }
    .grace-menu-item { display: flex; align-items: center; gap: 12px; padding: 10px 16px; color: #e2e8f0; cursor: pointer; transition: background 0.15s; }
    .grace-menu-item:hover { background: rgba(255,255,255,0.08); }
    .grace-menu-icon { width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; }
    .grace-menu-icon svg { width: 20px; height: 20px; }
    .grace-menu-divider { height: 1px; background: rgba(255,255,255,0.1); margin: 8px 0; }
    .grace-menu-power:hover { background: rgba(239,68,68,0.2); }

    /* File Manager */
    .grace-filemanager { display: flex; height: 100%; }
    .grace-fm-sidebar { width: 160px; background: #16161e; border-right: 1px solid rgba(255,255,255,0.05); padding: 8px 0; }
    .grace-fm-item { padding: 8px 16px; color: #94a3b8; cursor: pointer; font-size: 12px; }
    .grace-fm-item:hover { background: rgba(255,255,255,0.05); }
    .grace-fm-item.active { background: rgba(59,130,246,0.2); color: #60a5fa; }
    .grace-fm-main { flex: 1; padding: 16px; display: grid; grid-template-columns: repeat(auto-fill, 90px); gap: 8px; align-content: start; }
    .grace-fm-file { text-align: center; padding: 12px 8px; border-radius: 6px; cursor: pointer; font-size: 11px; color: #94a3b8; }
    .grace-fm-file:hover { background: rgba(255,255,255,0.05); }
    .grace-fm-file span { font-size: 32px; display: block; margin-bottom: 6px; }

    /* Settings */
    .grace-settings { padding: 20px; }
    .grace-settings-section { margin-bottom: 20px; }
    .grace-settings-section h3 { margin: 0 0 12px; color: #e2e8f0; font-size: 14px; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 8px; }
    .grace-settings label { display: block; margin: 8px 0; color: #94a3b8; font-size: 12px; }
    .grace-settings select { background: #2d2d3a; border: 1px solid rgba(255,255,255,0.1); color: #e2e8f0; padding: 4px 8px; border-radius: 4px; margin-left: 8px; }
    `;
    d.head.appendChild(style);
  }

  function launch() {
    ensureStyles();
    if (!container) init();
    container.style.display = 'block';
  }

  window.GraceDesktop = { launch };
})();
