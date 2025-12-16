import init, {
  System,
  GrubMenu,
  Memtest,
  NanoEditor,
  Desktop,
  neofetch_logo,
  start_doom,
  start_doom_with_difficulty,
  start_screensaver,
  doom_enable_procedural,
  doom_restore_original_map,
  mp_host,
  mp_join,
  mp_finalize,
  mp_id,
  mp_disconnect,
  fetch_http,
  curl_request,
  ping_request,
  dns_lookup,
  get_public_ip,
  start_idle_timer
} from './pkg/terminal_os.js';

import { state, setSystem, setGrubMenu, getState } from './js/state.js';
import { print } from './js/dom.js';
import { loadUserFiles, loadUserInfo } from './js/storage.js';
import { showGrub } from './js/grub.js';
import { initNano } from './js/nano.js';
import { initNeofetch } from './js/neofetch.js';
import { initTerminal } from './js/terminal.js';
import { initNetwork } from './js/network.js';

// Custom dialog system for Grace DE (replaces browser prompt/alert)
const GraceDialog = {
  _nextId: 1,
  _resolvers: {},
  
  // Show an alert dialog
  alert: (message, title = 'Alert') => {
    return new Promise((resolve) => {
      const id = GraceDialog._nextId++;
      GraceDialog._resolvers[id] = resolve;
      GraceDialog._show({
        id, title, message,
        type: 'alert',
        buttons: [{ text: 'OK', action: 'ok', primary: true }]
      });
    });
  },
  
  // Show a prompt dialog
  prompt: (message, defaultValue = '', title = 'Input') => {
    return new Promise((resolve) => {
      const id = GraceDialog._nextId++;
      GraceDialog._resolvers[id] = resolve;
      GraceDialog._show({
        id, title, message, defaultValue,
        type: 'prompt',
        buttons: [
          { text: 'Cancel', action: 'cancel' },
          { text: 'OK', action: 'ok', primary: true }
        ]
      });
    });
  },
  
  // Show a confirm dialog
  confirm: (message, title = 'Confirm') => {
    return new Promise((resolve) => {
      const id = GraceDialog._nextId++;
      GraceDialog._resolvers[id] = resolve;
      GraceDialog._show({
        id, title, message,
        type: 'confirm',
        buttons: [
          { text: 'Cancel', action: 'cancel' },
          { text: 'OK', action: 'ok', primary: true }
        ]
      });
    });
  },
  
  _show: (opts) => {
    // Append to grace-root or body to ensure it covers everything
    const container = document.querySelector('.grace-root') || document.body;
    
    const dialog = document.createElement('div');
    dialog.className = 's7-dialog-overlay';
    dialog.id = `s7-dialog-${opts.id}`;
    
    const inputHtml = opts.type === 'prompt' 
      ? `<input type="text" class="s7-dialog-input" id="s7-dialog-input-${opts.id}" value="${opts.defaultValue || ''}">`
      : '';
    
    const buttonsHtml = opts.buttons.map(b => 
      `<button class="s7-dialog-btn${b.primary ? ' s7-dialog-btn-primary' : ''}" data-action="${b.action}">${b.text}</button>`
    ).join('');
    
    dialog.innerHTML = `
      <div class="s7-dialog">
        <div class="s7-dialog-titlebar">${opts.title}</div>
        <div class="s7-dialog-body">
          <div class="s7-dialog-icon">&#9888;</div>
          <div class="s7-dialog-content">
            <div class="s7-dialog-message">${opts.message}</div>
            ${inputHtml}
          </div>
        </div>
        <div class="s7-dialog-buttons">${buttonsHtml}</div>
      </div>
    `;
    
    // Handle button clicks
    dialog.querySelectorAll('.s7-dialog-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const action = btn.dataset.action;
        const resolver = GraceDialog._resolvers[opts.id];
        delete GraceDialog._resolvers[opts.id];
        dialog.remove();
        
        if (action === 'ok') {
          if (opts.type === 'prompt') {
            const input = document.getElementById(`s7-dialog-input-${opts.id}`);
            resolver(input ? input.value : null);
          } else if (opts.type === 'confirm') {
            resolver(true);
          } else {
            resolver();
          }
        } else {
          resolver(opts.type === 'prompt' ? null : false);
        }
      });
    });
    
    container.appendChild(dialog);
    
    // Focus input if prompt
    if (opts.type === 'prompt') {
      const input = document.getElementById(`s7-dialog-input-${opts.id}`);
      if (input) {
        input.focus();
        input.select();
        input.addEventListener('keydown', (e) => {
          if (e.key === 'Enter') {
            dialog.querySelector('.s7-dialog-btn-primary')?.click();
          }
        });
      }
    }
  }
};

// Setup Grace Desktop bridge to expose functions to Rust
function setupGraceDesktopBridge() {
  const system = getState().system;
  
  // Expose filesystem test function for debugging
  window.testFS = () => {
    const sys = getState().system;
    if (!sys) {
      console.error('System not available');
      return;
    }
    
    console.log('Testing filesystem...');
    
    // Test mkdir
    console.log('Creating /home/user/testdir...');
    const mkdirResult = sys.fs_mkdir('/home/user/testdir');
    console.log('mkdir result:', mkdirResult);
    
    // Test write
    console.log('Writing /home/user/testdir/test.txt...');
    const writeResult = sys.fs_write('/home/user/testdir/test.txt', 'Hello World!');
    console.log('write result:', writeResult);
    
    // Test read
    console.log('Reading /home/user/testdir/test.txt...');
    const content = sys.fs_read('/home/user/testdir/test.txt');
    console.log('read result:', content);
    
    // Test list
    console.log('Listing /home/user/testdir...');
    const list = sys.fs_list('/home/user/testdir');
    console.log('list result:', list);
    
    // Test list /home/user
    console.log('Listing /home/user...');
    const list2 = sys.fs_list('/home/user');
    console.log('list result:', list2);
    
    return { mkdirResult, writeResult, content, list, list2 };
  };
  
  // Window drag state
  let dragState = { dragging: false, windowId: null, offsetX: 0, offsetY: 0 };
  
  // Setup global mouse handlers for dragging
  document.addEventListener('mousemove', (e) => {
    if (!dragState.dragging || !dragState.windowId) return;
    const win = document.getElementById(`s7-win-${dragState.windowId}`);
    if (!win) return;
    const x = e.clientX - dragState.offsetX;
    const y = Math.max(20, e.clientY - dragState.offsetY);
    win.style.left = x + 'px';
    win.style.top = y + 'px';
  });
  
  document.addEventListener('mouseup', () => {
    dragState.dragging = false;
    dragState.windowId = null;
  });
  
  // Click outside apple menu to close it
  document.addEventListener('click', (e) => {
    const dropdown = document.getElementById('s7-apple-dropdown');
    const appleMenu = document.querySelector('.s7-apple-menu');
    if (dropdown && dropdown.style.display !== 'none') {
      if (!dropdown.contains(e.target) && !appleMenu?.contains(e.target)) {
        dropdown.style.display = 'none';
      }
    }
  });
  
  window.GraceDesktop = {
    // Launch/hide
    launch: () => Desktop.launch(),
    hide: () => Desktop.hide(),
    
    // Window operations
    openTerminal: () => Desktop.open_terminal(),
    openFiles: () => Desktop.open_files(),
    openNotepad: () => Desktop.open_notepad(),
    openAbout: () => Desktop.open_about(),
    openTrash: () => Desktop.open_trash(),
    closeWindow: (id) => Desktop.close_window(id),
    
    // Window dragging
    startDrag: (windowId, e) => {
      const win = document.getElementById(`s7-win-${windowId}`);
      if (!win) return;
      const rect = win.getBoundingClientRect();
      dragState.dragging = true;
      dragState.windowId = windowId;
      dragState.offsetX = e.clientX - rect.left;
      dragState.offsetY = e.clientY - rect.top;
      // Bring to front
      win.style.zIndex = Date.now() % 10000 + 100;
    },
    
    // Menu
    toggleAppleMenu: () => Desktop.toggle_apple_menu(),
    shutdown: () => {
      Desktop.shutdown();
      document.getElementById('terminal').style.display = '';
    },
    
    // Terminal command handling
    handleTerminalCommand: (windowId) => {
      const input = document.getElementById(`s7-term-input-${windowId}`);
      const output = document.getElementById(`s7-term-out-${windowId}`);
      const promptEl = document.getElementById(`s7-term-prompt-${windowId}`);
      if (!input || !output) return;
      
      // Get fresh system reference
      const sys = getState().system;
      if (!sys) {
        console.error('System not available');
        return;
      }
      
      const cmd = input.value;
      input.value = '';
      
      if (cmd.trim()) {
        Desktop.add_terminal_history(cmd);
      }
      
      // Echo command
      if (cmd.trim() !== 'clear') {
        const line = document.createElement('div');
        line.className = 's7-term-line s7-term-cmd';
        line.textContent = `${promptEl ? promptEl.textContent : '$ '}${cmd}`;
        output.appendChild(line);
      }
      
      // Execute command
      const result = sys.exec(cmd);
      console.log('Command:', cmd);
      console.log('Result type:', typeof result);
      console.log('Result:', result);
      console.log('Result length:', result ? result.length : 0);
      
      // Handle escape sequences
      if (result === '\x1b[CLEAR]') {
        output.innerHTML = '';
      } else if (result === '\x1b[EXIT]') {
        window.GraceDesktop.printTerminal(windowId, 'logout', 's7-term-info');
      } else if (result === '\x1b[NEOFETCH_DATA]') {
        try {
          const logo = neofetch_logo(sys.exec('uname -n') || 'kpawnd');
          logo.split('\n').forEach(l => window.GraceDesktop.printTerminal(windowId, l, ''));
        } catch (e) {
          window.GraceDesktop.printTerminal(windowId, 'Neofetch failed', 's7-term-error');
        }
      } else if (result === '\x1b[PYTHON_REPL]') {
        window.GraceDesktop.printTerminal(windowId, 'Python 3.11.0 (sandboxed)', 's7-term-info');
        window.GraceDesktop.printTerminal(windowId, 'Type "exit()" to exit', 's7-term-info');
      } else if (result.startsWith('\x1b[OPEN:')) {
        // Open URL in new tab
        const url = result.slice(7, -1);
        window.open(url, '_blank');
        window.GraceDesktop.printTerminal(windowId, `Opening ${url}...`, 's7-term-info');
      } else if (result.startsWith('\x1b[LAUNCH_DOOM') || result.startsWith('\x1b[SCREENSAVER]')) {
        window.GraceDesktop.printTerminal(windowId, 'This command requires fullscreen terminal mode.', 's7-term-warn');
        window.GraceDesktop.printTerminal(windowId, 'Type "exit" to leave desktop, then run the command.', 's7-term-info');
      } else if (result.startsWith('\x1b[LAUNCH_GRACE]')) {
        window.GraceDesktop.printTerminal(windowId, 'Grace is already running', 's7-term-info');
      } else if (result.startsWith('\x1b[NANO:')) {
        const content = result.slice(7, -1);
        const colonIdx = content.indexOf(':');
        // Open notepad instead
        Desktop.open_notepad();
      } else if (result === '\x1b[REBOOT]') {
        window.GraceDesktop.printTerminal(windowId, 'Rebooting...', 's7-term-info');
        setTimeout(() => window.dispatchEvent(new CustomEvent('KP_REBOOT')), 500);
      } else if (result && result.trim()) {
        // Strip color codes and display - handles both [COLOR:x] and \x1b[... formats
        const clean = result
          .replace(/\[COLOR:[^\]]*\]/g, '')  // [COLOR:blue], [COLOR:reset], etc.
          .replace(/\x1b\[[0-9;]*m/g, '')    // Standard ANSI escapes
          .replace(/\x1b\[[A-Z_]+[^\]]*\]/g, ''); // Other escape sequences like \x1b[SOMETHING]
        console.log('Cleaned result:', clean);
        clean.split('\n').forEach(l => {
          if (l.trim()) window.GraceDesktop.printTerminal(windowId, l, '');
        });
      }
      
      // Update prompt
      if (promptEl) promptEl.textContent = sys.prompt();
      output.scrollTop = output.scrollHeight;
    },
    
    printTerminal: (windowId, text, cls) => {
      const output = document.getElementById(`s7-term-out-${windowId}`);
      if (!output) return;
      const line = document.createElement('div');
      line.className = 's7-term-line ' + (cls || '');
      line.textContent = text;
      output.appendChild(line);
      output.scrollTop = output.scrollHeight;
    },
    
    terminalHistoryUp: (windowId) => {
      const input = document.getElementById(`s7-term-input-${windowId}`);
      if (!input) return;
      const prev = Desktop.get_history_prev();
      if (prev !== undefined && prev !== null) {
        input.value = prev;
      }
    },
    
    terminalHistoryDown: (windowId) => {
      const input = document.getElementById(`s7-term-input-${windowId}`);
      if (!input) return;
      const next = Desktop.get_history_next();
      if (next !== undefined && next !== null) {
        input.value = next;
      }
    },
    
    // File manager operations
    refreshFileManager: (windowId) => {
      const path = Desktop.get_current_path() || '/home/user';
      window.GraceDesktop.fmNavigate(windowId, path);
    },
    
    fmNavigate: (windowId, path) => {
      Desktop.set_current_path(path);
      const list = document.getElementById(`s7-fm-list-${windowId}`);
      const pathEl = document.getElementById(`s7-fm-path-${windowId}`);
      const statusEl = document.getElementById(`s7-fm-status-${windowId}`);
      if (!list || !pathEl || !statusEl) return;
      
      // Get fresh system reference
      const sys = getState().system;
      
      pathEl.textContent = path;
      list.innerHTML = '';
      
      const entries = sys ? sys.fs_list(path) : [];
      let count = 0;
      
      // B&W folder and file icons
      const folderIcon = '<svg viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.5"><path d="M3 7h7l2-2h9v14H3z"/></svg>';
      const fileIcon = '<svg viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.5"><path d="M6 2h8l4 4v16H6z"/><path d="M14 2v4h4"/></svg>';
      
      if (Array.isArray(entries)) {
        entries.forEach(entry => {
          const name = entry.name || '';
          const isDir = !!entry.is_dir;
          if (!name) return;
          
          const item = document.createElement('div');
          item.className = 's7-fm-entry';
          item.innerHTML = `
            <div class="s7-fm-entry-icon">${isDir ? folderIcon : fileIcon}</div>
            <div class="s7-fm-entry-label">${name}</div>
          `;
          item.onclick = () => {
            list.querySelectorAll('.s7-fm-entry').forEach(e => e.classList.remove('selected'));
            item.classList.add('selected');
          };
          item.ondblclick = () => {
            if (isDir) {
              const next = path === '/' ? `/${name}` : `${path}/${name}`;
              window.GraceDesktop.fmNavigate(windowId, next);
            } else {
              window.GraceDesktop.fmOpenFile(windowId, path === '/' ? `/${name}` : `${path}/${name}`);
            }
          };
          list.appendChild(item);
          count++;
        });
      }
      
      statusEl.textContent = `${count} items`;
    },
    
    fmUp: (windowId) => {
      let path = Desktop.get_current_path();
      if (path === '/' || path === '') return;
      const parts = path.split('/').filter(Boolean);
      parts.pop();
      const parent = '/' + parts.join('/');
      window.GraceDesktop.fmNavigate(windowId, parent || '/');
    },
    
    fmOpenFile: (windowId, fullPath) => {
      const sys = getState().system;
      const content = sys ? sys.fs_read(fullPath) : '';
      // Open in notepad
      Desktop.open_notepad();
      // Slight delay to let window render
      setTimeout(() => {
        const notepadText = document.querySelector('.s7-notepad-text');
        const notepadPath = document.querySelector('.s7-notepad-path');
        if (notepadText) notepadText.value = content;
        if (notepadPath) notepadPath.textContent = fullPath;
      }, 100);
    },
    
    // Notepad operations
    notepadOpen: async (windowId) => {
      const path = await GraceDialog.prompt('Open file path:', '/home/user/', 'Open File');
      if (!path) return;
      const sys = getState().system;
      const content = sys ? sys.fs_read(path) : '';
      const textEl = document.getElementById(`s7-notepad-text-${windowId}`);
      const pathEl = document.getElementById(`s7-notepad-path-${windowId}`);
      if (textEl) textEl.value = content;
      if (pathEl) pathEl.textContent = path;
    },
    
    notepadSave: async (windowId) => {
      console.log('=== notepadSave START ===');
      console.log('windowId:', windowId);
      
      const textEl = document.getElementById(`s7-notepad-text-${windowId}`);
      const pathEl = document.getElementById(`s7-notepad-path-${windowId}`);
      
      console.log('Looking for elements:');
      console.log('  s7-notepad-text-' + windowId, '→', textEl);
      console.log('  s7-notepad-path-' + windowId, '→', pathEl);
      
      if (!textEl || !pathEl) {
        console.error('Elements not found!');
        alert('Error: Notepad elements not found for window ' + windowId);
        return;
      }
      
      const sys = getState().system;
      console.log('System object:', sys);
      
      if (!sys) {
        alert('Error: System not available');
        return;
      }
      
      let path = pathEl.textContent?.trim();
      console.log('Current path from element:', path);
      
      if (!path) {
        // Use browser prompt for reliability
        path = prompt('Save file as:', '/home/user/untitled.txt');
        console.log('User entered path:', path);
        if (!path) {
          console.log('User cancelled');
          return;
        }
      }
      
      // Ensure parent directories exist
      const parts = path.split('/').filter(Boolean);
      console.log('Path parts:', parts);
      
      if (parts.length > 1) {
        let parentPath = '';
        for (let i = 0; i < parts.length - 1; i++) {
          parentPath += '/' + parts[i];
          console.log('Ensuring parent dir:', parentPath);
          const mkdirResult = sys.fs_mkdir(parentPath);
          console.log('  mkdir result:', mkdirResult);
        }
      }
      
      const content = textEl.value;
      console.log('Content to save:', content.length, 'chars');
      console.log('Calling fs_write with path:', path);
      
      const success = sys.fs_write(path, content);
      console.log('fs_write returned:', success);
      
      if (success) {
        pathEl.textContent = path;
        alert('File saved to ' + path);
        console.log('=== notepadSave SUCCESS ===');
      } else {
        alert('Failed to save file to ' + path);
        console.log('=== notepadSave FAILED ===');
      }
    },
    
    notepadSaveAs: async (windowId) => {
      const textEl = document.getElementById(`s7-notepad-text-${windowId}`);
      const pathEl = document.getElementById(`s7-notepad-path-${windowId}`);
      if (!textEl || !pathEl) return;
      
      const currentPath = pathEl.textContent?.trim() || '/home/user/untitled.txt';
      const path = await GraceDialog.prompt('Save as:', currentPath, 'Save File As');
      if (!path) return;
      
      const sys = getState().system;
      if (sys) {
        // Ensure parent directories exist
        const parts = path.split('/').filter(Boolean);
        if (parts.length > 1) {
          let parentPath = '';
          for (let i = 0; i < parts.length - 1; i++) {
            parentPath += '/' + parts[i];
            sys.fs_mkdir(parentPath);
          }
        }
        const success = sys.fs_write(path, textEl.value);
        if (success) {
          pathEl.textContent = path;
          await GraceDialog.alert('File saved successfully!', 'Saved');
        } else {
          await GraceDialog.alert('Failed to save file.', 'Error');
        }
      }
    },
    
    // File manager operations
    fmNewFolder: async (windowId) => {
      console.log('=== fmNewFolder START ===');
      
      const name = prompt('Enter folder name:', 'New Folder');
      console.log('User entered name:', name);
      if (!name) return;
      
      const sys = getState().system;
      console.log('System:', sys);
      if (!sys) {
        alert('System not available');
        return;
      }
      
      const currentPath = Desktop.get_current_path() || '/home/user';
      console.log('Current path:', currentPath);
      
      const newPath = currentPath === '/' ? `/${name}` : `${currentPath}/${name}`;
      console.log('New folder path:', newPath);
      
      const result = sys.fs_mkdir(newPath);
      console.log('fs_mkdir result:', result);
      
      if (result) {
        alert('Folder created: ' + newPath);
        window.GraceDesktop.fmNavigate(windowId, currentPath);
      } else {
        alert('Failed to create folder. It may already exist.');
      }
      console.log('=== fmNewFolder END ===');
    },
    
    fmDelete: async (windowId) => {
      const list = document.getElementById(`s7-fm-list-${windowId}`);
      const selected = list?.querySelector('.s7-fm-entry.selected');
      if (!selected) {
        await GraceDialog.alert('No file or folder selected.', 'Delete');
        return;
      }
      
      const name = selected.querySelector('.s7-fm-entry-label')?.textContent;
      if (!name) return;
      
      const confirmed = await GraceDialog.confirm(`Delete "${name}"?`, 'Confirm Delete');
      if (!confirmed) return;
      
      const sys = getState().system;
      const currentPath = Desktop.get_current_path() || '/home/user';
      const fullPath = currentPath === '/' ? `/${name}` : `${currentPath}/${name}`;
      
      if (sys && sys.fs_rm(fullPath, true)) {
        window.GraceDesktop.fmNavigate(windowId, currentPath);
      } else {
        await GraceDialog.alert('Failed to delete.', 'Error');
      }
    },
  };
}

// entry point
async function main() {
  try {
    await init();
    
    // Initialize WASM modules in JS modules
    initNano({ NanoEditor });
    initNeofetch({ neofetch_logo });
    initTerminal({ 
      start_doom, 
      start_doom_with_difficulty, 
      start_screensaver, 
      doom_enable_procedural, 
      doom_restore_original_map,
      mp_host,
      mp_join,
      mp_finalize,
      mp_id,
      mp_disconnect
    });
    initNetwork({ fetch_http, curl_request, ping_request, dns_lookup, get_public_ip });
    // Multiplayer handled fully in Rust now; use console to connect:
    // doom_multiplayer_connect('ws://localhost:8081')
    
    // Create system instances
    setSystem(new System());
    setGrubMenu(new GrubMenu());
    
    // Setup Grace Desktop bridge
    setupGraceDesktopBridge();
    
    // Restore user files and user info from localStorage
    loadUserFiles();
    loadUserInfo();
    
    // Start GRUB bootloader
    showGrub();
  } catch (error) {
    document.getElementById('grub').style.display = 'none';
    document.getElementById('terminal').style.display = 'flex';
    print(`Failed to load: ${error.message}`, 'error');
  }
}

main();
setTimeout(() => start_idle_timer(60000), 1000);

// Handle reboot by reinitializing System and GRUB, preserving user files
window.addEventListener('KP_REBOOT', async () => {
  try {
    // Hide Grace desktop if visible
    const graceRoot = document.querySelector('.grace-root');
    if (graceRoot) graceRoot.style.display = 'none';
    // Keep current System to preserve destructive changes; refresh GRUB menu only
    setGrubMenu(new GrubMenu());
    // Hide terminal and graphics, clear output
    document.getElementById('terminal').style.display = 'none';
    document.getElementById('graphics').style.display = 'none';
    document.getElementById('output').innerHTML = '';
    // Show GRUB and restart boot sequence
    document.getElementById('grub').style.display = 'flex';
    showGrub();
  } catch (e) {
    print(`Reboot failed: ${e.message}`, 'error');
  }
});
