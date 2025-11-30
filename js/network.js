import { print, scrollToBottom } from './dom.js';

// These will be set by main.js after WASM init
let fetch_http, curl_request, ping_request, dns_lookup, get_public_ip;

export function initNetwork(wasm) {
  fetch_http = wasm.fetch_http;
  curl_request = wasm.curl_request;
  ping_request = wasm.ping_request;
  dns_lookup = wasm.dns_lookup;
  get_public_ip = wasm.get_public_ip;
}

export async function fetchUrl(url) {
  print(`Fetching ${url}...`, 'info');
  try {
    print(await fetch_http(url), 'output');
  } catch (e) {
    print(`Error: ${e.message || e}`, 'error');
  }
  scrollToBottom();
}

export async function doCurl(url, method, showHeaders) {
  print(`* Connecting to ${url}...`, 'info');
  try {
    (await curl_request(url, method, showHeaders)).split('\n').forEach(line => print(line, 'output'));
  } catch (e) {
    print(`curl: (7) Failed to connect: ${e.message || e}`, 'error');
  }
  scrollToBottom();
}

export async function doPing(host) {
  let url = host.startsWith('http') ? host : `https://${host}`;
  print(`PING ${host}`, 'info');

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

  if (results.length) {
    const min = Math.min(...results).toFixed(1);
    const max = Math.max(...results).toFixed(1);
    const avg = (results.reduce((a, b) => a + b, 0) / results.length).toFixed(1);
    print(`\n--- ${host} ping statistics ---`, 'info');
    print(`4 packets transmitted, ${results.length} received, ${((4 - results.length) / 4 * 100).toFixed(0)}% packet loss`, 'output');
    print(`rtt min/avg/max = ${min}/${avg}/${max} ms`, 'output');
  }
  scrollToBottom();
}

export async function doDns(host) {
  print(`; <<>> DiG 9.18.0 <<>> ${host}`, 'info');
  print(`;; Using DNS-over-HTTPS (Cloudflare)`, 'info');
  print('', 'output');
  try {
    print(`;; ANSWER SECTION:`, 'info');
    (await dns_lookup(host)).split('\n').filter(Boolean).forEach(line => print(line, 'output'));
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
