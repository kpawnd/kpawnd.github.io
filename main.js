import init, {
  System,
  GrubMenu,
  NanoEditor,
  start_doom,
  start_doom_with_difficulty,
  start_screensaver,
  doom_enable_procedural,
  doom_restore_original_map,
  fetch_http,
  curl_request,
  ping_request,
  dns_lookup,
  get_public_ip,
  start_idle_timer
} from './pkg/terminal_os.js';

import { getState, setSystem, setGrubMenu } from './js/state.js';
import { print } from './js/dom.js';
import { loadUserFiles, loadUserInfo } from './js/storage.js';
import { showGrub } from './js/grub.js';
import { showBiosScreen } from './js/bios.js';
import { initNano } from './js/nano.js';
import { initTerminal } from './js/terminal.js';
import { initNetwork } from './js/network.js';

async function main() {
  try {
    await init();

    initNano({ NanoEditor });
    initTerminal({
      start_doom,
      start_doom_with_difficulty,
      start_screensaver,
      doom_enable_procedural,
      doom_restore_original_map
    });
    initNetwork({ fetch_http, curl_request, ping_request, dns_lookup, get_public_ip });

    const system = new System();
    setSystem(system);
    setGrubMenu(new GrubMenu());

    await system.init();

    loadUserFiles();
    loadUserInfo();

    showBiosScreen(() => {
      showGrub();
    }, { system });
  } catch (error) {
    const grub = document.getElementById('grub');
    const terminal = document.getElementById('terminal');
    if (grub) grub.style.display = 'none';
    if (terminal) terminal.style.display = 'flex';
    print(`Failed to load: ${error.message}`, 'error');
  }
}

main();
setTimeout(() => start_idle_timer(60000), 1000);

window.addEventListener('beforeunload', async () => {
  const system = getState().system;
  if (system && typeof system.save === 'function') {
    try {
      await system.save();
    } catch (error) {
      console.warn('Failed to save system state:', error);
    }
  }
});

window.addEventListener('KP_REBOOT', async () => {
  try {
    const currentSystem = getState().system;
    if (currentSystem && typeof currentSystem.save === 'function') {
      try {
        await currentSystem.save();
      } catch (error) {
        console.warn('Failed to save system state before reboot:', error);
      }
    }

    setGrubMenu(new GrubMenu());

    const terminal = document.getElementById('terminal');
    const graphics = document.getElementById('graphics');
    const output = document.getElementById('output');
    const grub = document.getElementById('grub');

    if (terminal) terminal.style.display = 'none';
    if (graphics) graphics.style.display = 'none';
    if (output) output.innerHTML = '';
    if (grub) grub.style.display = 'none';

    showBiosScreen(() => {
      showGrub();
    }, { system: currentSystem });
  } catch (error) {
    print(`Reboot failed: ${error.message}`, 'error');
  }
});