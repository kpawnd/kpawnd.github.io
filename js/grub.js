import { state } from './state.js';
import { print, getElement } from './dom.js';
import { beginBoot } from './boot.js';

export function showGrub() {
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
        clearInterval(state.grubInterval);
        document.removeEventListener('keydown', handleGrubKey);
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
  grubPre.innerHTML = state.grubMenu.render()
    .replace(/\x1b\[HIGHLIGHT\]/g, '<span class="grub-selected">')
    .replace(/\x1b\[NORMAL\]/g, '</span>');
}

function bootSelected() {
  const selected = state.grubMenu.get_selected();
  getElement('grub').style.display = 'none';
  getElement('terminal').style.display = 'flex';

  if (selected === 2) {
    print('Entering UEFI Firmware Settings...', 'info');
    print('(Simulated - no real firmware access)', 'info');
    print('', 'output');
    setTimeout(() => {
      getElement('output').innerHTML = '';
      beginBoot();
    }, 2000);
  } else {
    beginBoot();
  }
}
