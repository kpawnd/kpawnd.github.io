const HOLD_MS = 7000;

function nowBiosDateString() {
  const d = new Date();
  const two = (n) => String(n).padStart(2, '0');
  const yy = two(d.getFullYear() % 100);
  const mm = two(d.getMonth() + 1);
  const dd = two(d.getDate());
  const hh = two(d.getHours());
  const mi = two(d.getMinutes());
  const ss = two(d.getSeconds());
  return `${mm}/${dd}/${yy} ${hh}:${mi}:${ss}`;
}

function detectCpuModel() {
  const ua = navigator.userAgent || '';
  const cores = navigator.hardwareConcurrency || 1;
  if (/Windows/i.test(ua)) return `x86_64 Processor (${cores} cores)`;
  if (/Macintosh|Mac OS/i.test(ua)) return `Apple-compatible Processor (${cores} cores)`;
  if (/Linux/i.test(ua)) return `Linux-compatible Processor (${cores} cores)`;
  return `Generic Processor (${cores} cores)`;
}

function detectCpuSpeedMhz() {
  const cores = Math.max(1, navigator.hardwareConcurrency || 1);
  return String(1800 + cores * 200);
}

function buildAutoProfile(totalMemoryKb) {
  const tz = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  const locale = navigator.language || 'en-US';
  return {
    vendor: 'AMIBIOS',
    biosVersion: '08.00.15',
    cpuModel: detectCpuModel(),
    cpuSpeedMhz: detectCpuSpeedMhz(),
    usbSummary: `${Math.max(2, navigator.hardwareConcurrency || 2)} Device(s)`,
    sataPort1: `VFS_DISK_${locale}`,
    sataPort2: `VFS_DISK_${tz.replace(/\//g, '_')}`,
    totalMemoryKb: Number(totalMemoryKb) || 0
  };
}

function createBiosLines(profile) {
  const oemCode = '62-0100-001131-00101111-040226-440BX';
  return [
    `AMIBIOS(C)2026 American Megatrends, Inc.`,
    `${profile.vendor} Version ${profile.biosVersion}`,
    `BIOS Date: ${nowBiosDateString()}`,
    '',
    `CPU : ${profile.cpuModel}`,
    `Speed : ${profile.cpuSpeedMhz}MHz`,
    '',
    `Checking NVRAM...`,
    `Initializing USB Controllers .. Done.`,
    `USB Devices: ${profile.usbSummary}`,
    `Memory Test : ${profile.totalMemoryKb}K OK`,
    '',
    'Detected ATA/ATAPI Devices...',
    `SATA Port1 : ${profile.sataPort1}`,
    `SATA Port2 : ${profile.sataPort2}`,
    '',
    'Press DEL to run Setup',
    'Press F8 for BBS POPUP',
    '',
    `BootBlock BIOS v1.0`,
    `Verifying DMI Pool Data ........`,
    '',
    'Boot from Hard Disk...',
    '',
    oemCode
  ];
}

function setScreenVisible(id, visible, display = 'block') {
  const el = document.getElementById(id);
  if (!el) return;
  el.style.display = visible ? display : 'none';
}

export function showBiosScreen(onComplete, options = {}) {
  const system = options.system;
  const totalMemoryKb = system && typeof system.memory_total_kb === 'function'
    ? system.memory_total_kb()
    : 0;
  const profile = buildAutoProfile(totalMemoryKb);
  const biosLines = createBiosLines(profile);

  const bios = document.getElementById('bios');
  const pre = document.querySelector('#bios pre');
  if (!bios || !pre) {
    onComplete();
    return;
  }

  setScreenVisible('terminal', false);
  setScreenVisible('grub', false);
  setScreenVisible('bios', true);
  pre.textContent = biosLines.join('\n');

  window.setTimeout(() => {
    setScreenVisible('bios', false);
    onComplete();
  }, HOLD_MS);
}
