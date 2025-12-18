import { state } from './state.js';
import { print, scrollToBottom, getElement } from './dom.js';
import { setupTerminal } from './terminal.js';

export function beginBoot() {
  // Clear screen before booting
  getElement('output').innerHTML = '';

  // Use the new modular boot system
  const bootMessages = state.system.boot_simulate_sequence();
  drainBootLines(bootMessages, 0);
}

function drainBootLines(messages, index) {
  if (index >= messages.length) {
    // Boot complete
    setTimeout(() => {
      if (state.system.post_boot_clear_needed()) {
        state.system.acknowledge_post_boot();
        getElement('output').innerHTML = '';
      }
      setupTerminal();
    }, 2000);
    return;
  }

  const line = messages[index];
  if (line !== '') {
    print(line, 'boot');
    scrollToBottom();
  }

  setTimeout(() => drainBootLines(messages, index + 1), 80);
}
