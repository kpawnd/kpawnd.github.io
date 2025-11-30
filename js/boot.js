import { state } from './state.js';
import { print, scrollToBottom, getElement } from './dom.js';
import { setupTerminal } from './terminal.js';

export function beginBoot() {
  state.system.start_boot();
  drainBootLines();
}

function drainBootLines() {
  const line = state.system.next_boot_line();
  if (line === null || line === undefined) return;

  if (line !== '') {
    print(line, 'boot');
    scrollToBottom();
  }

  if (line.includes('BOOT_COMPLETE')) {
    setTimeout(() => {
      if (state.system.post_boot_clear_needed()) {
        state.system.acknowledge_post_boot();
        getElement('output').innerHTML = '';
      }
      setupTerminal();
    }, 2000);
  } else {
    setTimeout(drainBootLines, 80);
  }
}
