use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, WebSocket};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    Tcp,
    Udp,
    WebSocket,
    Http,
    Icmp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketState {
    Closed,
    Connecting,
    Open,
    Closing,
    Listen,
    TimeWait,
    Established,
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ipv4: String,
    pub ipv6: String,
    pub mac: String,
    pub mtu: u32,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub is_up: bool,
    pub is_loopback: bool,
}

#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
}

#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub destination: String,
    pub gateway: String,
    pub genmask: String,
    pub flags: String,
    pub iface: String,
}

pub struct Socket {
    pub id: u32,
    pub protocol: Protocol,
    pub state: SocketState,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub url: Option<String>,
    pub ws: Option<WebSocket>,
}

impl Socket {
    pub fn new(id: u32, protocol: Protocol) -> Self {
        Socket {
            id,
            protocol,
            state: SocketState::Closed,
            local_addr: "0.0.0.0".to_string(),
            local_port: 0,
            remote_addr: "0.0.0.0".to_string(),
            remote_port: 0,
            url: None,
            ws: None,
        }
    }

    pub fn connect_ws(&mut self, url: &str) -> Result<(), String> {
        if self.protocol != Protocol::WebSocket {
            return Err("Socket is not a WebSocket".to_string());
        }

        match WebSocket::new(url) {
            Ok(ws) => {
                ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
                self.ws = Some(ws);
                self.url = Some(url.to_string());
                self.state = SocketState::Connecting;
                self.local_port = 0;
                let (host, port) = parse_remote_endpoint(url);
                self.remote_addr = host;
                self.remote_port = port;
                Ok(())
            }
            Err(e) => Err(format!("Failed to create WebSocket: {:?}", e)),
        }
    }

    pub fn send(&self, data: &str) -> Result<(), String> {
        if let Some(ws) = &self.ws {
            ws.send_with_str(data)
                .map_err(|e| format!("Failed to send: {:?}", e))
        } else {
            Err("No active WebSocket connection".to_string())
        }
    }

    pub fn close(&mut self) -> Result<(), String> {
        if let Some(ws) = &self.ws {
            ws.close()
                .map_err(|e| format!("Failed to close: {:?}", e))?;
            self.state = SocketState::Closing;
        }
        Ok(())
    }
}

fn parse_remote_endpoint(url: &str) -> (String, u16) {
    let lower = url.to_lowercase();
    let default_port = if lower.starts_with("wss://") { 443 } else { 80 };
    let without_scheme = if let Some(pos) = url.find("://") {
        &url[pos + 3..]
    } else {
        url
    };
    let host_port = without_scheme.split('/').next().unwrap_or(without_scheme);

    if let Some((host, port_str)) = host_port.rsplit_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            return (host.to_string(), port);
        }
    }

    (host_port.to_string(), default_port)
}

pub struct NetworkStack {
    sockets: HashMap<u32, Socket>,
    next_socket_id: u32,
}

impl Default for NetworkStack {
    fn default() -> Self {
        NetworkStack::new()
    }
}

impl NetworkStack {
    pub fn new() -> Self {
        NetworkStack {
            sockets: HashMap::new(),
            next_socket_id: 1,
        }
    }

    pub fn socket(&mut self, protocol: Protocol) -> u32 {
        let id = self.next_socket_id;
        self.next_socket_id += 1;
        self.sockets.insert(id, Socket::new(id, protocol));
        id
    }

    pub fn connect_ws(&mut self, socket_id: u32, url: &str) -> Result<(), String> {
        if let Some(socket) = self.sockets.get_mut(&socket_id) {
            socket.connect_ws(url)
        } else {
            Err("Invalid socket ID".to_string())
        }
    }

    pub fn send(&self, socket_id: u32, data: &str) -> Result<(), String> {
        if let Some(socket) = self.sockets.get(&socket_id) {
            socket.send(data)
        } else {
            Err("Invalid socket ID".to_string())
        }
    }

    pub fn close(&mut self, socket_id: u32) -> Result<(), String> {
        if let Some(socket) = self.sockets.get_mut(&socket_id) {
            socket.close()?;
            Ok(())
        } else {
            Err("Invalid socket ID".to_string())
        }
    }

    pub fn list_sockets(&self) -> Vec<String> {
        let mut result = Vec::new();
        for socket in self.sockets.values() {
            let proto = match socket.protocol {
                Protocol::Tcp => "tcp",
                Protocol::Udp => "udp",
                Protocol::WebSocket => "tcp",
                Protocol::Http => "tcp",
                Protocol::Icmp => "icmp",
            };
            let state = match socket.state {
                SocketState::Closed => "CLOSED",
                SocketState::Connecting => "SYN_SENT",
                SocketState::Open => "ESTABLISHED",
                SocketState::Closing => "FIN_WAIT1",
                SocketState::Listen => "LISTEN",
                SocketState::TimeWait => "TIME_WAIT",
                SocketState::Established => "ESTABLISHED",
            };
            let local = format!("{}:{}", socket.local_addr, socket.local_port);
            let remote = format!("{}:{}", socket.remote_addr, socket.remote_port);
            result.push(format!(
                "{:<6} {:>6} {:>6} {:<23} {:<23} {}",
                proto, 0, 0, local, remote, state
            ));
        }
        result
    }

    pub fn get_interfaces(&self) -> Vec<NetworkInterface> {
        Vec::new()
    }

    pub fn get_routes(&self) -> Vec<RouteEntry> {
        Vec::new()
    }

    pub fn dns_lookup(&self, hostname: &str) -> Vec<DnsRecord> {
        let _ = hostname;
        Vec::new()
    }

    pub fn ping_host(&self, host: &str, _seq: u32) -> (f64, u8) {
        let _ = host;
        (0.0, 0)
    }

    pub fn traceroute_hops(&self, host: &str) -> Vec<(u8, String, f64)> {
        let _ = host;
        Vec::new()
    }

    pub fn arp_table(&self) -> Vec<(String, String, String)> {
        Vec::new()
    }

    pub async fn http_get(url: &str) -> Result<String, String> {
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(url, &opts)
            .map_err(|e| format!("Failed to create request: {:?}", e))?;

        let window = web_sys::window().ok_or("No window object")?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| format!("Fetch failed: {:?}", e))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| "Response is not a Response object")?;

        let text = JsFuture::from(
            resp.text()
                .map_err(|e| format!("Failed to get text: {:?}", e))?,
        )
        .await
        .map_err(|e| format!("Failed to read text: {:?}", e))?;

        text.as_string()
            .ok_or_else(|| "Response text is not a string".to_string())
    }

    pub async fn http_post(url: &str, body: &str) -> Result<String, String> {
        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        opts.set_body(&JsValue::from_str(body));

        let request = Request::new_with_str_and_init(url, &opts)
            .map_err(|e| format!("Failed to create request: {:?}", e))?;

        request
            .headers()
            .set("Content-Type", "application/json")
            .map_err(|e| format!("Failed to set header: {:?}", e))?;

        let window = web_sys::window().ok_or("No window object")?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| format!("Fetch failed: {:?}", e))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| "Response is not a Response object")?;

        let text = JsFuture::from(
            resp.text()
                .map_err(|e| format!("Failed to get text: {:?}", e))?,
        )
        .await
        .map_err(|e| format!("Failed to read text: {:?}", e))?;

        text.as_string()
            .ok_or_else(|| "Response text is not a string".to_string())
    }
}

// Export HTTP functions for WASM
#[wasm_bindgen]
pub async fn fetch_http(url: &str) -> Result<String, JsValue> {
    NetworkStack::http_get(url)
        .await
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub async fn post_http(url: &str, body: &str) -> Result<String, JsValue> {
    NetworkStack::http_post(url, body)
        .await
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub async fn curl_request(url: &str, method: &str, show_headers: bool) -> Result<String, JsValue> {
    let start = js_sys::Date::now();

    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|e| JsValue::from_str(&format!("curl: {:?}", e)))?;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| JsValue::from_str(&format!("curl: (7) Failed to connect: {:?}", e)))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| JsValue::from_str("curl: Invalid response"))?;

    let elapsed = js_sys::Date::now() - start;
    let status = resp.status();
    let status_text = resp.status_text();

    let mut output = String::new();

    if show_headers {
        output.push_str(&format!("HTTP/1.1 {} {}\n", status, status_text));

        // Get headers
        let headers = resp.headers();
        if let Ok(Some(ct)) = headers.get("content-type") {
            output.push_str(&format!("content-type: {}\n", ct));
        }
        if let Ok(Some(s)) = headers.get("server") {
            output.push_str(&format!("server: {}\n", s));
        }
        if let Ok(Some(d)) = headers.get("date") {
            output.push_str(&format!("date: {}\n", d));
        }
        output.push_str(&format!("\n* Request completed in {:.0}ms\n", elapsed));
    } else {
        let text = JsFuture::from(
            resp.text()
                .map_err(|e| JsValue::from_str(&format!("curl: {:?}", e)))?,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("curl: {:?}", e)))?;

        if let Some(body) = text.as_string() {
            output.push_str(&body);
        }
    }

    Ok(output)
}

/// no-cors mode for compatibility
#[wasm_bindgen]
pub async fn ping_request(url: &str) -> Result<String, JsValue> {
    let start = js_sys::Date::now();

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::NoCors); // Use no-cors for ping

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|e| JsValue::from_str(&format!("ping: {:?}", e)))?;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;

    match JsFuture::from(window.fetch_with_request(&request)).await {
        Ok(_) => {
            // With no-cors we get an opaque response but timing is still valid.
            let elapsed = js_sys::Date::now() - start;
            Ok(format!("time={:.1}ms", elapsed))
        }
        Err(_) => Err(JsValue::from_str("timeout")),
    }
}

/// DNS lookup via DNS-over-HTTPS (Cloudflare)
#[wasm_bindgen]
pub async fn dns_lookup(hostname: &str) -> Result<String, JsValue> {
    let url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type=A",
        hostname
    );

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| JsValue::from_str(&format!("DNS error: {:?}", e)))?;

    request
        .headers()
        .set("Accept", "application/dns-json")
        .map_err(|e| JsValue::from_str(&format!("DNS error: {:?}", e)))?;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| JsValue::from_str(&format!("DNS query failed: {:?}", e)))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| JsValue::from_str("Invalid DNS response"))?;

    let json = JsFuture::from(
        resp.json()
            .map_err(|e| JsValue::from_str(&format!("DNS error: {:?}", e)))?,
    )
    .await
    .map_err(|e| JsValue::from_str(&format!("DNS parse error: {:?}", e)))?;

    // Parse the JSON response
    let mut output = String::new();

    if let Ok(answers) = js_sys::Reflect::get(&json, &JsValue::from_str("Answer")) {
        if let Some(arr) = answers.dyn_ref::<js_sys::Array>() {
            for i in 0..arr.length() {
                if let Some(answer) = arr.get(i).dyn_ref::<js_sys::Object>() {
                    let name = js_sys::Reflect::get(answer, &JsValue::from_str("name"))
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();
                    let rtype = js_sys::Reflect::get(answer, &JsValue::from_str("type"))
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as u32;
                    let data = js_sys::Reflect::get(answer, &JsValue::from_str("data"))
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();
                    let ttl = js_sys::Reflect::get(answer, &JsValue::from_str("TTL"))
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as u32;

                    let type_str = match rtype {
                        1 => "A",
                        28 => "AAAA",
                        5 => "CNAME",
                        15 => "MX",
                        16 => "TXT",
                        _ => "UNKNOWN",
                    };

                    output.push_str(&format!(
                        "{} has {} record {} (TTL: {})\n",
                        name, type_str, data, ttl
                    ));
                }
            }
        }
    }

    if output.is_empty() {
        output = format!("No DNS records found for {}", hostname);
    }

    Ok(output)
}

/// Get public IP address
#[wasm_bindgen]
pub async fn get_public_ip() -> Result<String, JsValue> {
    let url = "https://api.ipify.org?format=json";

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|e| JsValue::from_str(&format!("Error: {:?}", e)))?;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to get IP: {:?}", e)))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| JsValue::from_str("Invalid response"))?;

    let json = JsFuture::from(
        resp.json()
            .map_err(|e| JsValue::from_str(&format!("Error: {:?}", e)))?,
    )
    .await
    .map_err(|e| JsValue::from_str(&format!("Parse error: {:?}", e)))?;

    let ip = js_sys::Reflect::get(&json, &JsValue::from_str("ip"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(ip)
}
