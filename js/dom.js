const COLOR_MAP = {
  reset: null, black: '#000', red: '#f00', green: '#0f0',
  yellow: '#ff0', blue: '#00f', magenta: '#f0f', cyan: '#0ff',
  white: '#fff', gray: '#888', grey: '#888'
};

export function escapeHtml(text) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

export function renderColorTokens(raw) {
  const parts = [];
  let open = false;
  let lastIndex = 0;
  const regex = /\x1b\[COLOR:([#0-9A-Fa-f]+|[a-z]+)\]/g;
  let match;

  while ((match = regex.exec(raw)) !== null) {
    parts.push(escapeHtml(raw.slice(lastIndex, match.index)));
    if (open) {
      parts.push('</span>');
      open = false;
    }

    const colorValue = match[1];
    const cssColor = colorValue.startsWith('#') ? colorValue : COLOR_MAP[colorValue];

    if (cssColor) {
      parts.push(`<span style="color:${cssColor}">`);
      open = true;
    }
    lastIndex = regex.lastIndex;
  }

  parts.push(escapeHtml(raw.slice(lastIndex)));
  if (open) parts.push('</span>');
  return parts.join('');
}

export function print(text, className = '') {
  const output = document.getElementById('output');
  const line = document.createElement('div');
  line.className = `line ${className}`;
  if (className.includes('boot')) {
    line.innerHTML = renderBootLine(text);
  } else {
    line.innerHTML = text.includes('\x1b[COLOR:') ? renderColorTokens(text) : escapeHtml(text);
  }
  output.appendChild(line);
}

export function scrollToBottom() {
  const term = document.getElementById('terminal');
  term.scrollTop = term.scrollHeight;
}

export function getElement(id) {
  return document.getElementById(id);
}

function renderBootLine(raw) {
  const escRemoved = raw.replace(/\x1b\[[^m]*m/g, '');
  const m = /^\[\s*(OK|FAILED|ERROR|WARN|WARNING)\s*\]\s*(.*)$/.exec(escRemoved);
  if (!m) {
    return escapeHtml(escRemoved);
  }
  const status = m[1].toUpperCase();
  const msg = m[2];
  let cls = 'ok';
  if (status === 'FAILED' || status === 'ERROR') cls = 'fail';
  else if (status === 'WARN' || status === 'WARNING') cls = 'warn';
  const statusHtml = `<span class="boot-status ${cls}">[ ${escapeHtml(status)} ]</span>`;
  return `${statusHtml} ${escapeHtml(msg)}`;
}
