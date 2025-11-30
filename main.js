import init, {
  System,
  GrubMenu,
  Memtest,
  NanoEditor,
  neofetch_logo,
  start_doom,
  start_screensaver,
  fetch_http,
  curl_request,
  ping_request,
  dns_lookup,
  get_public_ip,
  start_idle_timer
} from './pkg/terminal_os.js';

import { state, setSystem, setGrubMenu } from './js/state.js';
import { print } from './js/dom.js';
import { loadUserFiles, loadUserInfo } from './js/storage.js';
import { showGrub } from './js/grub.js';
import { initNano } from './js/nano.js';
import { initNeofetch } from './js/neofetch.js';
import { initTerminal } from './js/terminal.js';
import { initNetwork } from './js/network.js';

// entry point
async function main() {
  try {
    await init();
    
    // Initialize WASM modules in JS modules
    initNano({ NanoEditor });
    initNeofetch({ neofetch_logo });
    initTerminal({ start_doom, start_screensaver });
    initNetwork({ fetch_http, curl_request, ping_request, dns_lookup, get_public_ip });
    
    // Create system instances
    setSystem(new System());
    setGrubMenu(new GrubMenu());
    
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
