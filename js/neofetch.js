import { print, scrollToBottom } from './dom.js';

let neofetch_logo;

export function initNeofetch(wasm) {
  neofetch_logo = wasm.neofetch_logo;
}

export function displayNeofetch() {
  const ua = navigator.userAgent;
  const info = {
    os: 'Unknown',
    hostname: 'localhost',
    user: 'root',
    kernel: 'Unknown',
    browser: 'Unknown',
    cpu: `${navigator.hardwareConcurrency || 'Unknown'} cores`,
    memory: 'Unknown',
    resolution: `${window.innerWidth}x${window.innerHeight}`,
    uptime: Math.floor(performance.now() / 1000)
  };

  // Detect OS
  if (ua.includes('Win')) info.os = 'Windows';
  else if (ua.includes('Mac')) info.os = 'macOS';
  else if (ua.includes('Linux')) info.os = 'Linux';
  else if (ua.includes('Android')) info.os = 'Android';
  else if (/iOS|iPhone|iPad/.test(ua)) info.os = 'iOS';

  // Detect browser
  if (ua.includes('Firefox')) info.browser = 'Firefox';
  else if (ua.includes('Edg')) info.browser = 'Edge';
  else if (ua.includes('Chrome')) info.browser = 'Chrome';
  else if (ua.includes('Safari')) info.browser = 'Safari';

  // Detect architecture
  if (/x64|x86_64|Win64/.test(ua)) info.kernel = 'x86_64';
  else if (/ARM|aarch64/.test(ua)) info.kernel = 'ARM64';
  else if (ua.includes('x86')) info.kernel = 'x86';

  // Memory estimation
  if (navigator.deviceMemory) {
    info.memory = `${navigator.deviceMemory} GB`;
  } else if (performance.memory?.jsHeapSizeLimit) {
    const heapGB = performance.memory.jsHeapSizeLimit / (1024 ** 3);
    info.memory = `~${Math.ceil(heapGB * 4)} GB (estimated)`;
  }

  // Format uptime
  const hours = Math.floor(info.uptime / 3600);
  const mins = Math.floor((info.uptime % 3600) / 60);
  const uptimeStr = hours > 0 ? `${hours} hours, ${mins} mins` : `${mins} mins`;

  const logoBlock = neofetch_logo(info.os).split('\n');
  const infoLines = [
    `${info.user}@${info.hostname}`,
    '─'.repeat(`${info.user}@${info.hostname}`.length),
    `OS: ${info.os}`,
    `Host: ${info.browser}`,
    `Kernel: ${info.kernel}`,
    `Uptime: ${uptimeStr}`,
    `Shell: kpawnd-sh`,
    `Resolution: ${info.resolution}`,
    `Terminal: ${info.browser}`,
    `CPU: ${info.cpu}`,
    ...(info.memory !== 'Unknown' ? [`Memory: ${info.memory}`] : [])
  ];

  const visibleWidth = (s) => s.replace(/\x1b\[COLOR:#[0-9A-Fa-f]{6}\]/g, '').length;
  const maxLogoWidth = Math.max(...logoBlock.map(visibleWidth));

  for (let i = 0; i < Math.max(logoBlock.length, infoLines.length); i++) {
    const l = logoBlock[i] || '';
    const r = infoLines[i] || '';
    print(l + ' '.repeat(maxLogoWidth - visibleWidth(l) + 3) + r, 'output');
  }
  scrollToBottom();
}
