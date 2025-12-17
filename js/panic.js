import { print, scrollToBottom } from './dom.js';

let panicTimeout = null;

export function showKernelPanic(panicMessage = null) {
  const output = document.getElementById('output');
  const prompt = document.getElementById('prompt');
  output.innerHTML = '';
  prompt.style.display = 'none';

  document.body.style.backgroundColor = '#000';
  print('', 'output');

  if (panicMessage) {
    // Display the actual panic message from the backend
    const lines = panicMessage.split('\n');
    lines.forEach(line => print(line, 'output panic-text'));
  } else {
    // Fallback to the old hardcoded panic (for compatibility)
    print('Kernel panic - not syncing: Attempted to kill init!', 'output panic-text');
    print('exitcode=0x00000000', 'output panic-text');
    print('', 'output');

    const addr = () => (Math.random() * 0xffffffff >>> 0).toString(16).padStart(8, '0');
    const traces = [
      `[<${addr()}>] panic+0x1a8/0x350`,
      `[<${addr()}>] do_exit+0xb92/0xbd0`,
      `[<${addr()}>] do_group_exit+0x39/0xa0`,
      `[<${addr()}>] __x64_sys_exit_group+0x14/0x20`,
      `[<${addr()}>] do_syscall_64+0x5c/0xa0`,
      `[<${addr()}>] entry_SYSCALL_64_after_hwframe+0x44/0xae`
    ];
    traces.forEach(t => print(t, 'output panic-text'));

    print('', 'output');
    print('---[ end Kernel panic - not syncing: Attempted to kill init! ]---', 'output panic-text');
  }

  print('', 'output');
  print('Rebooting in 10 seconds...', 'output blink');
  scrollToBottom();

  // Add CSS for panic if not already present
  if (!document.getElementById('panic-style')) {
    const style = document.createElement('style');
    style.id = 'panic-style';
    style.textContent = `
      .panic-text { color: #ff3333 !important; }
      .blink { animation: blink-anim 1s step-end infinite; }
      @keyframes blink-anim {
        50% { opacity: 0; }
      }
    `;
    document.head.appendChild(style);
  }

  panicTimeout = setTimeout(() => {
    location.reload();
  }, 10000);
}

export function cancelPanic() {
  if (panicTimeout) {
    clearTimeout(panicTimeout);
    panicTimeout = null;
  }
}
