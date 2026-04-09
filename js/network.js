import { print, scrollToBottom } from './dom.js';

// These will be set by main.js after WASM init
let fetch_http, curl_request, ping_request, dns_lookup, get_public_ip;

function sanitizeTarget(raw) {
  // Strip common shell quoting/trailing punctuation that can leak into host args.
  return (raw || '').trim().replace(/^['"`]+|['"`]+$/g, '').replace(/[\s'"`]+$/g, '');
}

function normalizeUrl(raw, { preferHttps = true } = {}) {
  const target = sanitizeTarget(raw);
  if (!target) return '';

  if (/^https?:\/\//i.test(target)) {
    return target;
  }

  const isIpv4 = /^\d{1,3}(\.\d{1,3}){3}$/.test(target);
  const scheme = preferHttps && !isIpv4 ? 'https://' : 'http://';
  return `${scheme}${target}`;
}

export function initNetwork(wasm) {
  fetch_http = wasm.fetch_http;
  curl_request = wasm.curl_request;
  ping_request = wasm.ping_request;
  dns_lookup = wasm.dns_lookup;
  get_public_ip = wasm.get_public_ip;
}

export async function fetchUrl(url) {
  const normalized = normalizeUrl(url, { preferHttps: true });
  if (!normalized) {
    print('Error: missing URL', 'error');
    scrollToBottom();
    return;
  }

  print(`Fetching ${normalized}...`, 'info');
  try {
    print(await fetch_http(normalized), 'output');
  } catch (e) {
    print(`Error: ${e.message || e}`, 'error');
  }
  scrollToBottom();
}

export async function doCurl(url, method, showHeaders) {
  const normalized = normalizeUrl(url, { preferHttps: false });
  if (!normalized) {
    print('curl: no URL specified', 'error');
    scrollToBottom();
    return;
  }

  print(`* Connecting to ${normalized}...`, 'info');
  try {
    (await curl_request(normalized, method, showHeaders)).split('\n').forEach(line => print(line, 'output'));
  } catch (e) {
    print(`curl: (7) Failed to connect: ${e.message || e}`, 'error');
  }
  scrollToBottom();
}

export async function doPing(host) {
  const target = sanitizeTarget(host);
  const url = normalizeUrl(target, { preferHttps: true });

  if (!target || !url) {
    print('ping: missing host operand', 'error');
    scrollToBottom();
    return;
  }

  print(`PING ${target}`, 'info');

  const results = [];
  for (let i = 0; i < 4; i++) {
    try {
      const result = await ping_request(url);
      print(`seq=${i + 1}: ${result}`, 'output');
      const match = result.match(/time=([0-9.]+)ms/);
      if (match) results.push(parseFloat(match[1]));
    } catch {
      print(`seq=${i + 1}: timeout`, 'error');
    }
    await new Promise(r => setTimeout(r, 200));
  }

  print(`\n--- ${target} ping statistics ---`, 'info');
  print(`4 packets transmitted, ${results.length} received, ${((4 - results.length) / 4 * 100).toFixed(0)}% packet loss`, 'output');

  if (results.length) {
    const min = Math.min(...results).toFixed(1);
    const max = Math.max(...results).toFixed(1);
    const avg = (results.reduce((a, b) => a + b, 0) / results.length).toFixed(1);
    print(`rtt min/avg/max = ${min}/${avg}/${max} ms`, 'output');
  } else {
    print('rtt min/avg/max = 0.0/0.0/0.0 ms', 'output');
  }
  scrollToBottom();
}

export async function doDns(host) {
  const target = sanitizeTarget(host);
  if (!target) {
    print('DNS lookup failed: missing hostname', 'error');
    scrollToBottom();
    return;
  }

  print(`; <<>> DiG 9.18.0 <<>> ${target}`, 'info');
  print(`;; Using DNS-over-HTTPS (Cloudflare)`, 'info');
  print('', 'output');
  try {
    print(`;; ANSWER SECTION:`, 'info');
    (await dns_lookup(target)).split('\n').filter(Boolean).forEach(line => print(line, 'output'));
  } catch (e) {
    print(`DNS lookup failed: ${e.message || e}`, 'error');
  }
  scrollToBottom();
}

export async function doMyIp() {
  try {
    print(await get_public_ip(), 'output');
  } catch (e) {
    print(`Failed to get IP: ${e.message || e}`, 'error');
  }
  scrollToBottom();
}
