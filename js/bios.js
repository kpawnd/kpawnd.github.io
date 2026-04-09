const HOLD_MS = 7000;
const PROBE_TIMEOUT_MS = 250;
const DEVICE_ESTIMATE_CACHE_KEY = 'kp_bios_device_estimate_v1';

function timeoutAfter(ms, fallbackValue) {
  return new Promise((resolve) => {
    window.setTimeout(() => resolve(fallbackValue), ms);
  });
}

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
  const deviceMemoryGb = Math.max(2, Number(navigator.deviceMemory || 0));
  return String(1500 + cores * 180 + deviceMemoryGb * 120);
}

function fnv1a32(input) {
  let hash = 0x811c9dc5;
  for (let i = 0; i < input.length; i += 1) {
    hash ^= input.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return hash >>> 0;
}

function seededPick(items, seed, shift = 0) {
  if (!Array.isArray(items) || items.length === 0) return '';
  return items[(seed >>> shift) % items.length];
}

function getWebglRenderer() {
  try {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
    if (!gl) return 'unknown-gpu';
    const ext = gl.getExtension('WEBGL_debug_renderer_info');
    if (!ext) return 'webgl-gpu';
    const renderer = gl.getParameter(ext.UNMASKED_RENDERER_WEBGL);
    return renderer || 'webgl-gpu';
  } catch {
    return 'unknown-gpu';
  }
}

function getHardwareSignals(totalMemoryKb) {
  const tz = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  const ua = navigator.userAgent || 'unknown-ua';
  const platform = navigator.platform || 'unknown-platform';
  const vendor = navigator.vendor || 'unknown-vendor';
  const locale = navigator.language || 'en-US';
  const cores = Math.max(1, navigator.hardwareConcurrency || 1);
  const memoryGb = Math.max(1, Number(navigator.deviceMemory || 0));
  const screenInfo = `${window.screen?.width || 0}x${window.screen?.height || 0}x${window.screen?.colorDepth || 0}`;
  const gpu = getWebglRenderer();

  return {
    ua,
    platform,
    vendor,
    locale,
    tz,
    cores,
    memoryGb,
    totalMemoryKb: Number(totalMemoryKb) || 0,
    screenInfo,
    gpu,
    cpuModel: detectCpuModel(),
    cpuSpeedMhz: detectCpuSpeedMhz()
  };
}

function createBiosIdentity(signals) {
  const fingerprint = [
    signals.ua,
    signals.platform,
    signals.vendor,
    signals.locale,
    signals.tz,
    signals.cores,
    signals.memoryGb,
    signals.screenInfo,
    signals.gpu,
    signals.totalMemoryKb
  ].join('|');
  const seed = fnv1a32(fingerprint);

  const biosVersion = [
    5 + (seed % 5),
    String((seed >>> 6) % 100).padStart(2, '0'),
    String((seed >>> 14) % 100).padStart(2, '0')
  ].join('.');

  const brand = [
    String((seed >>> 1) % 100).padStart(2, '0'),
    String((seed >>> 9) % 10000).padStart(4, '0')
  ].join('');

  const dateStamp = nowBiosDateString().slice(0, 8).replace(/\//g, '');
  const oemCode = [
    String((seed >>> 2) % 100).padStart(2, '0'),
    String((seed >>> 11) % 10000).padStart(4, '0'),
    String(((seed ^ 0xa5a5a5a5) >>> 0) % 100000000).padStart(8, '0'),
    String(((seed ^ 0x5a5a5a5a) >>> 0) % 100000000).padStart(8, '0'),
    dateStamp,
    String((seed >>> 3) % 1000000).padStart(6, '0')
  ].join('-');

  return {
    vendorName: `${signals.vendor || 'OEM'} Firmware`,
    biosVersion,
    brand,
    oemCode
  };
}

async function getStorageProfile(seed) {
  let quotaBytes = 0;
  try {
    if (navigator.storage && typeof navigator.storage.estimate === 'function') {
      const estimate = await navigator.storage.estimate();
      quotaBytes = Number(estimate?.quota || 0);
    }
  } catch {
    // Ignore blocked storage estimate API.
  }

  const minGiB = 64;
  const maxGiB = 2048;
  const estimatedGiB = quotaBytes > 0
    ? Math.max(minGiB, Math.min(maxGiB, Math.floor(quotaBytes / (1024 ** 3))))
    : 128 + ((seed >>> 7) % 512);

  const family = seededPick(['SN', 'PM', 'MX', 'KC', 'WD', 'EVO', 'XG'], seed, 3);
  const suffix = String(100 + ((seed >>> 13) % 900));
  const sataPort1 = `${family}${suffix} SSD ${estimatedGiB}GB`;

  const hasSecondary = ((seed >>> 5) & 0x1) === 1;
  const sataPort2 = hasSecondary
    ? `${seededPick(['ST', 'TOS', 'MAT'], seed, 17)} DVD-ROM`
    : 'Not Detected';

  return { sataPort1, sataPort2 };
}

async function getGrantedCount(getter) {
  try {
    const devices = await Promise.race([
      getter(),
      timeoutAfter(PROBE_TIMEOUT_MS, null)
    ]);
    return Array.isArray(devices) ? devices.length : null;
  } catch {
    return null;
  }
}

async function detectConnectedDeviceStats() {
  const usbCount = navigator.usb && typeof navigator.usb.getDevices === 'function'
    ? await getGrantedCount(() => navigator.usb.getDevices())
    : null;

  const hidCount = navigator.hid && typeof navigator.hid.getDevices === 'function'
    ? await getGrantedCount(() => navigator.hid.getDevices())
    : null;

  const serialCount = navigator.serial && typeof navigator.serial.getPorts === 'function'
    ? await getGrantedCount(() => navigator.serial.getPorts())
    : null;

  let gamepadCount = 0;

  try {
    if (typeof navigator.getGamepads === 'function') {
      const pads = Array.from(navigator.getGamepads() || []).filter(Boolean);
      gamepadCount = pads.length;
    }
  } catch {
    // Ignore unsupported gamepad API.
  }

  const trustedBusCount = [usbCount, hidCount, serialCount]
    .filter((v) => Number.isInteger(v) && v >= 0)
    .reduce((sum, v) => sum + v, 0);

  return { trustedBusCount, gamepadCount };
}

async function buildAutoProfile(totalMemoryKb) {
  const signals = getHardwareSignals(totalMemoryKb);
  const identity = createBiosIdentity(signals);
  const storage = await getStorageProfile(fnv1a32([identity.oemCode, signals.platform, signals.ua].join('|')));
  const stats = await detectConnectedDeviceStats();

  // Calibrate to old POST-era wording: prefer concrete granted-device counts,
  // blend in broad I/O signals only as a soft estimate when direct buses are absent.
  const fallbackBaseline = 0;
  const estimatedUsbDevices = stats.trustedBusCount > 0
    ? stats.trustedBusCount
    : stats.gamepadCount > 0
      ? stats.gamepadCount
      : fallbackBaseline;

  let stableDeviceEstimate = estimatedUsbDevices;
  try {
    const cachedValue = Number(window.localStorage.getItem(DEVICE_ESTIMATE_CACHE_KEY));
    if (estimatedUsbDevices === 0) {
      stableDeviceEstimate = 0;
    } else if (Number.isFinite(cachedValue) && cachedValue >= 0) {
      // Keep output stable while adapting up or down over time.
      stableDeviceEstimate = Math.max(0, Math.round((cachedValue * 0.65) + (estimatedUsbDevices * 0.35)));
    }
    window.localStorage.setItem(DEVICE_ESTIMATE_CACHE_KEY, String(stableDeviceEstimate));
  } catch {
    // Ignore storage access restrictions.
  }

  return {
    vendor: identity.vendorName,
    biosVersion: identity.biosVersion,
    cpuModel: signals.cpuModel,
    cpuSpeedMhz: signals.cpuSpeedMhz,
    usbSummary: `${stableDeviceEstimate} Device(s)`,
    sataPort1: storage.sataPort1,
    sataPort2: storage.sataPort2,
    totalMemoryKb: signals.totalMemoryKb,
    biosBrand: identity.brand,
    oemCode: identity.oemCode
  };
}

function createBiosLines(profile) {
  return [
    `${profile.biosBrand} BIOS(C)2026 Compatible Firmware`,
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
    profile.oemCode
  ];
}

function setScreenVisible(id, visible, display = 'block') {
  const el = document.getElementById(id);
  if (!el) return;
  el.style.display = visible ? display : 'none';
}

function applyResponsiveBiosLayout(bios, pre, logo) {
  if (!bios || !pre) return;

  const vw = Math.max(320, window.innerWidth || 0);
  const vh = Math.max(240, window.innerHeight || 0);

  if (logo) {
    const targetLogoWidth = Math.min(vw * 0.34, 620);
    logo.style.width = `${Math.round(targetLogoWidth)}px`;
  }

  const logoBottom = logo ? (logo.getBoundingClientRect().bottom - bios.getBoundingClientRect().top) : 0;
  const topPadding = Math.max(Math.round(vh * 0.12), Math.round(logoBottom + 18));
  const sidePadding = Math.max(10, Math.round(vw * 0.012));

  pre.style.paddingTop = `${topPadding}px`;
  pre.style.paddingLeft = `${sidePadding}px`;
  pre.style.paddingRight = `${sidePadding}px`;
}

export function showBiosScreen(onComplete, options = {}) {
  (async () => {
    const system = options.system;
    const totalMemoryKb = system && typeof system.memory_total_kb === 'function'
      ? system.memory_total_kb()
      : 0;
    const profile = await buildAutoProfile(totalMemoryKb);
    const biosLines = createBiosLines(profile);

    const bios = document.getElementById('bios');
    const pre = document.querySelector('#bios pre');
    const logo = document.getElementById('bios-logo');
    if (!bios || !pre) {
      onComplete();
      return;
    }

    setScreenVisible('terminal', false);
    setScreenVisible('grub', false);
    setScreenVisible('bios', true);
    applyResponsiveBiosLayout(bios, pre, logo);
    window.requestAnimationFrame(() => applyResponsiveBiosLayout(bios, pre, logo));
    pre.textContent = biosLines.join('\n');

    const onResize = () => applyResponsiveBiosLayout(bios, pre, logo);
    window.addEventListener('resize', onResize);

    window.setTimeout(() => {
      window.removeEventListener('resize', onResize);
      setScreenVisible('bios', false);
      onComplete();
    }, HOLD_MS);
  })();
}
