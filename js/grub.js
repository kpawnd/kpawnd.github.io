import { state } from './state.js';
import { print, getElement } from './dom.js';
import { beginBoot } from './boot.js';
import { setMemtest } from './state.js';
import { Memtest } from '../pkg/terminal_os.js';

export function showGrub() {
  // Clear terminal before showing GRUB
  const output = document.getElementById('output');
  if (output) output.innerHTML = '';

  const grubDiv = getElement('grub');
  grubDiv.style.display = 'flex';
  // If GRUB was removed, show rescue error instead of menu
  try {
    if (!state.system.has_grub()) {
      const grubPre = document.querySelector('#grub pre');
      grubPre.textContent = "error: file '/boot/grub/grub.cfg' not found.\nEntering rescue mode...\ngrub rescue>";
      return;
    }
  } catch (e) {
    // If backend not ready, proceed with menu
  }
  updateGrubDisplay();

  const handleGrubKey = (e) => {
    if (state.grubMenu.is_edit_mode()) {
      if (e.key === 'Escape') {
        e.preventDefault();
        state.grubMenu.exit_special_mode();
        updateGrubDisplay();
      } else if ((e.ctrlKey && e.key === 'x') || e.key === 'F10') {
        e.preventDefault();
        state.grubMenu.exit_special_mode();
        bootSelected();
      }
      return;
    }

    if (state.grubMenu.is_cmdline_mode()) {
      if (e.key === 'Escape') {
        e.preventDefault();
        state.grubMenu.exit_special_mode();
        updateGrubDisplay();
      }
      return;
    }

    switch (e.key) {
      case 'ArrowUp':
        e.preventDefault();
        state.grubMenu.move_up();
        updateGrubDisplay();
        break;
      case 'ArrowDown':
        e.preventDefault();
        state.grubMenu.move_down();
        updateGrubDisplay();
        break;
      case 'Enter':
        e.preventDefault();
        const selected = state.grubMenu.get_selected();
        const inAdvanced = state.grubMenu.is_advanced_mode();
        
        // Check if this selection will actually boot (not just navigate menus)
        const willBoot = (inAdvanced && selected !== 0) || (!inAdvanced && selected !== 1);
        
        if (willBoot) {
          clearInterval(state.grubInterval);
          document.removeEventListener('keydown', handleGrubKey);
        }
        
        bootSelected();
        break;
      case 'e':
        e.preventDefault();
        state.grubMenu.enter_edit_mode();
        updateGrubDisplay();
        break;
      case 'c':
        e.preventDefault();
        state.grubMenu.enter_cmdline_mode();
        updateGrubDisplay();
        break;
    }
  };

  document.addEventListener('keydown', handleGrubKey);

  state.grubInterval = setInterval(() => {
    const shouldContinue = state.grubMenu.tick();
    updateGrubDisplay();

    if (!shouldContinue || state.grubMenu.should_boot()) {
      clearInterval(state.grubInterval);
      document.removeEventListener('keydown', handleGrubKey);
      bootSelected();
    }
  }, 1000);
}

function updateGrubDisplay() {
  const grubPre = document.querySelector('#grub pre');
  const currentBootloader = state.system.boot_get_current_bootloader();
  let display = state.grubMenu.render();

  // Add current bootloader info at the top
  const bootloaderInfo = `Current Bootloader: ${currentBootloader.toUpperCase()}\n\n`;
  display = bootloaderInfo + display;

  grubPre.innerHTML = display
    .replace(/\x1b\[HIGHLIGHT\]/g, '<span class="grub-selected">')
    .replace(/\x1b\[NORMAL\]/g, '</span>');
}

function bootSelected() {
  const selected = state.grubMenu.get_selected();

  if (state.grubMenu.is_advanced_mode()) {
    // In advanced mode
    if (selected === 0) {
      // Back to main menu
      state.grubMenu.exit_advanced_mode();
      updateGrubDisplay();
      return; // Don't boot, just return to main menu
    } else {
      // Boot selected option
      getElement('grub').style.display = 'none';
      getElement('terminal').style.display = 'flex';
      if (selected === 3) {
        startMemtest();
      } else {
        beginBoot();
      }
      return;
    }
  } else {
    // In main menu
    if (selected === 0) {
      // Normal boot
      getElement('grub').style.display = 'none';
      getElement('terminal').style.display = 'flex';
      beginBoot();
      return;
    } else if (selected === 1) {
      // Advanced options - enter submenu
      state.grubMenu.enter_advanced_mode();
      updateGrubDisplay();
      return; // Don't boot, stay in GRUB
    } else if (selected === 2) {
      // Memory test
      getElement('grub').style.display = 'none';
      getElement('terminal').style.display = 'flex';
      startMemtest();
      return;
    }
  }
}

function startMemtest() {
  // Get system memory size (assume 512MB for simulation)
  const memSize = 512;
  const memtest = new Memtest(memSize);
  setMemtest(memtest);

  // Clear terminal and show header
  getElement('output').innerHTML = '';
  print(memtest.get_header(), 'info');

  // Start memtest loop
  const memtestInterval = setInterval(() => {
    const continueTesting = memtest.tick();
    const line = memtest.get_current_line();
    print(line, 'info');

    if (!continueTesting) {
      clearInterval(memtestInterval);
      print('\nPress ESC to return to GRUB menu', 'info');

      // Listen for ESC to return to GRUB
      const handleMemtestKey = (e) => {
        if (e.key === 'Escape') {
          document.removeEventListener('keydown', handleMemtestKey);
          getElement('terminal').style.display = 'none';
          getElement('grub').style.display = 'flex';
          showGrub();
        }
      };
      document.addEventListener('keydown', handleMemtestKey);
    }
  }, 500); // Update every 500ms for visible progress
}
