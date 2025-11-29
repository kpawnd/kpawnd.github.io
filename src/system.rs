use crate::{
    kernel::Kernel,
    network::{NetworkStack, Protocol},
    process::{Priority, ProcState},
    python::PythonInterpreter,
    services::ServiceManager,
    shell::{prompt, Shell},
    vfs::Inode,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct System {
    kernel: Kernel,
    shell: Shell,
    network: NetworkStack,
    services: ServiceManager,
    booted: bool,
    cleared_after_boot: bool,
    python_interp: Option<PythonInterpreter>,
    in_python_repl: bool,
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl System {
    #[wasm_bindgen(constructor)]
    pub fn new() -> System {
        let mut system = System {
            kernel: Kernel::new(),
            shell: Shell::new(),
            network: NetworkStack::new(),
            services: ServiceManager::new(),
            booted: false,
            cleared_after_boot: false,
            python_interp: None,
            in_python_repl: false,
        };

        // Auto-start system services
        system
            .services
            .auto_start_services(&mut |name| system.kernel.proc.spawn(name, 1));

        system
    }
    #[wasm_bindgen]
    pub fn start_boot(&mut self) {
        self.kernel.generate_boot_log();
    }
    #[wasm_bindgen]
    pub fn next_boot_line(&mut self) -> Option<String> {
        if let Some(l) = self.kernel.next_boot_line() {
            if l.contains("BOOT_COMPLETE") {
                self.booted = true;
            }
            Some(l)
        } else {
            None
        }
    }
    #[wasm_bindgen]
    pub fn is_booted(&self) -> bool {
        self.booted
    }
    #[wasm_bindgen]
    pub fn post_boot_clear_needed(&self) -> bool {
        self.booted && !self.cleared_after_boot
    }
    #[wasm_bindgen]
    pub fn acknowledge_post_boot(&mut self) {
        self.cleared_after_boot = true;
    }
    #[wasm_bindgen]
    pub fn prompt(&self) -> String {
        prompt(&self.kernel)
    }

    #[wasm_bindgen]
    pub fn exec(&mut self, line: &str) -> String {
        self.kernel.tick();
        self.kernel.scheduler.tick(&mut self.kernel.proc);
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            self.shell.history.push(trimmed.into());
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return String::new();
        }
        let cmd = parts[0];
        let args = &parts[1..];
        if self.shell.registry.has(cmd) {
            let pid = self.kernel.proc.spawn(cmd, 1);
            self.kernel.scheduler.add(pid, Priority::Normal);
        }
        match cmd {
            "echo" => { let out=args.join(" "); if out=="github" { format!("\x1b[OPEN:{}]", self.shell.env.get("GITHUB").unwrap()) } else { out } }
            "help" => "arp cat cd clear curl dig doom echo exit free help history host hostname id ifconfig ip kill ls mkdir myip nc neofetch netstat nslookup ping ps pwd rm route screensaver service socket ss touch traceroute uname uptime wget".into(),
            "neofetch" => "\x1b[NEOFETCH_DATA]".to_string(),
            "python" => { if args.is_empty() { self.start_python_repl() } else { "python: script execution not supported".to_string() } }
            "doom" => "\x1b[LAUNCH_DOOM]".to_string(),
            "screensaver" => "\x1b[LAUNCH_SCREENSAVER]".to_string(),
            "wget" => self.cmd_wget(args),
            "curl" => self.cmd_curl(args),
            "myip" => self.cmd_myip(),
            "ls" => self.cmd_ls(args),
            "cd" => self.cmd_cd(args),
            "pwd" => self.kernel.fs.cwd.clone(),
            "cat" => self.cmd_cat(args),
            "touch" => self.cmd_touch(args),
            "mkdir" => self.cmd_mkdir(args),
            "rm" => self.cmd_rm(args),
            "clear" => "\x1b[CLEAR]".into(),
            "exit" => "\x1b[EXIT]".into(),
            "ps" => self.cmd_ps(),
            "kill" => self.cmd_kill(args),
            "uname" => self.cmd_uname(args),
            "hostname" => self.cmd_hostname(),
            "id" => "uid=1000(user) gid=1000(user)".into(),
            "whoami" => "user".into(),
            "uptime" => format!("up {}ms", self.kernel.uptime_ms()),
            "free" => self.cmd_free(),
            "history" => self.cmd_history(),
            "env" => self.cmd_env(),
            "export" => self.cmd_export(args),
            "netstat" => self.cmd_netstat(args),
            "ss" => self.cmd_ss(args),
            "socket" => self.cmd_socket(args),
            "service" => self.cmd_service(args),
            "ping" => self.cmd_ping(args),
            "traceroute" | "tracert" => self.cmd_traceroute(args),
            "ifconfig" => self.cmd_ifconfig(args),
            "ip" => self.cmd_ip(args),
            "route" => self.cmd_route(args),
            "arp" => self.cmd_arp(args),
            "host" | "nslookup" | "dig" => self.cmd_host(args),
            "nc" | "netcat" => self.cmd_nc(args),
            "" => String::new(),
            _ => format!("sh: {}: command not found", cmd),
        }
    }

    fn cmd_ls(&self, args: &[&str]) -> String {
        let path = args.first().unwrap_or(&".");
        match self.kernel.fs.resolve(path) {
            Some(node) if node.is_dir => {
                let mut names: Vec<_> = node.children.keys().collect();
                names.sort();
                names
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("  ")
            }
            Some(node) => node.name.clone(),
            None => format!("ls: {}: no such file or directory", path),
        }
    }
    fn cmd_cd(&mut self, args: &[&str]) -> String {
        let target = args.first().unwrap_or(&"/");
        match self.kernel.fs.cd(target) {
            Ok(()) => String::new(),
            Err(e) => format!("cd: {}: {}", target, e),
        }
    }
    fn cmd_cat(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return String::new();
        }
        match self.kernel.fs.resolve(args[0]) {
            Some(n) if !n.is_dir => n.data.clone(),
            Some(_) => format!("cat: {}: is a directory", args[0]),
            None => format!("cat: {}: no such file", args[0]),
        }
    }
    fn cmd_touch(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return String::new();
        }
        let cwd = self.kernel.fs.cwd.clone();
        if let Some(dir) = self.kernel.fs.resolve_mut(&cwd) {
            let name = args[0];
            if !dir.children.contains_key(name) {
                dir.children.insert(name.into(), Inode::file(name, ""));
            }
        }
        String::new()
    }
    fn cmd_mkdir(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return String::new();
        }
        let cwd = self.kernel.fs.cwd.clone();
        if let Some(dir) = self.kernel.fs.resolve_mut(&cwd) {
            let name = args[0];
            if dir.children.contains_key(name) {
                return format!("mkdir: {}: exists", name);
            }
            dir.children.insert(name.into(), Inode::dir(name));
        }
        String::new()
    }
    fn cmd_rm(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return String::new();
        }
        let cwd = self.kernel.fs.cwd.clone();
        if let Some(dir) = self.kernel.fs.resolve_mut(&cwd) {
            if dir.children.remove(args[0]).is_none() {
                return format!("rm: {}: no such file", args[0]);
            }
        }
        String::new()
    }
    fn cmd_ps(&self) -> String {
        let mut out = String::from("  PID  PPID STAT CMD\n");
        for p in self.kernel.proc.list() {
            let st = match p.state {
                ProcState::Run => "R",
                ProcState::Sleep => "S",
                ProcState::Stop => "T",
                ProcState::Zombie => "Z",
            };
            out.push_str(&format!("{:5} {:5} {}    {}\n", p.pid, p.ppid, st, p.name));
        }
        out
    }
    fn cmd_kill(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: kill <pid>".into();
        }
        match args[0].parse::<u32>() {
            Ok(pid) => {
                if self.kernel.proc.kill(pid) {
                    String::new()
                } else {
                    format!("kill: {}: no such process or cannot kill", pid)
                }
            }
            Err(_) => "kill: invalid pid".into(),
        }
    }
    fn cmd_uname(&self, args: &[&str]) -> String {
        if args.contains(&"-a") {
            format!("kpawnd {} wasm32 kpawnd", crate::kernel::VERSION)
        } else {
            "kpawnd".into()
        }
    }
    fn cmd_hostname(&self) -> String {
        self.kernel
            .fs
            .resolve("/etc/hostname")
            .map(|n| n.data.clone())
            .unwrap_or_else(|| "localhost".into())
    }
    fn cmd_free(&self) -> String {
        let (used, total) = self.kernel.mem.usage();
        format!(
            "total: {}K\nused:  {}K\nfree:  {}K",
            total / 1024,
            used / 1024,
            (total - used) / 1024
        )
    }
    fn cmd_history(&self) -> String {
        self.shell
            .history
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{:4}  {}", i + 1, c))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn cmd_env(&self) -> String {
        let mut vars: Vec<_> = self.shell.env.iter().collect();
        vars.sort_by_key(|(k, _)| *k);
        vars.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn cmd_export(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return self.cmd_env();
        }
        for arg in args {
            if let Some((k, v)) = arg.split_once('=') {
                self.shell.env.insert(k.into(), v.into());
            }
        }
        String::new()
    }

    fn cmd_wget(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: wget [options] <url>\n  -O <file>  write to file\n  -q         quiet mode".to_string();
        }
        format!("\x1b[FETCH:{}]", args.last().unwrap_or(&""))
    }

    fn cmd_curl(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "curl: try 'curl --help' for more information".to_string();
        }
        let mut url = "";
        let mut method = "GET";
        let mut show_headers = false;
        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "-I" | "--head" => show_headers = true,
                "-X" => {
                    if i + 1 < args.len() {
                        method = args[i + 1];
                        i += 1;
                    }
                }
                "-H" | "--header" => i += 1, // Skip header value
                "-d" | "--data" => i += 1,   // Skip data value
                "-o" | "-O" => i += 1,       // Skip output file
                "--help" => {
                    return "Usage: curl [options] <url>\n  -I, --head     Show headers only\n  -X <method>    HTTP method\n  -H <header>    Add header\n  -d <data>      POST data\n  -o <file>      Output to file".to_string();
                }
                s if !s.starts_with('-') => url = s,
                _ => {}
            }
            i += 1;
        }
        if url.is_empty() {
            return "curl: no URL specified".to_string();
        }
        // Return escape sequence for real curl request
        format!("\x1b[CURL:{}:{}:{}]", method, show_headers, url)
    }

    fn cmd_netstat(&self, args: &[&str]) -> String {
        let show_all = args.contains(&"-a");
        let show_listening = args.contains(&"-l");
        let show_tcp = args.contains(&"-t") || args.is_empty();
        let show_udp = args.contains(&"-u");
        let show_numeric = args.contains(&"-n");

        let mut out = String::from("Active Internet connections");
        if show_listening {
            out.push_str(" (only servers)");
        } else if show_all {
            out.push_str(" (servers and established)");
        }
        out.push_str(
            "\nProto Recv-Q Send-Q Local Address           Foreign Address         State\n",
        );

        // Add some simulated listening sockets
        if show_all || show_listening {
            if show_tcp {
                out.push_str(
                    "tcp        0      0 0.0.0.0:22              0.0.0.0:*               LISTEN\n",
                );
                out.push_str(
                    "tcp        0      0 0.0.0.0:80              0.0.0.0:*               LISTEN\n",
                );
                out.push_str(
                    "tcp        0      0 127.0.0.1:631           0.0.0.0:*               LISTEN\n",
                );
            }
            if show_udp {
                out.push_str("udp        0      0 0.0.0.0:68              0.0.0.0:*                           \n");
                out.push_str("udp        0      0 0.0.0.0:5353            0.0.0.0:*                           \n");
            }
        }

        // Add actual sockets
        for socket_line in self.network.list_sockets() {
            out.push_str(&socket_line);
            out.push('\n');
        }

        let _ = (show_numeric, show_tcp, show_udp); // Silence unused warnings
        out
    }

    fn cmd_ss(&self, args: &[&str]) -> String {
        let show_all = args.contains(&"-a");
        let show_listening = args.contains(&"-l");
        let show_tcp = args.contains(&"-t") || args.is_empty();
        let show_numeric = args.contains(&"-n");

        let mut out = String::from(
            "Netid  State      Recv-Q Send-Q Local Address:Port    Peer Address:Port\n",
        );

        if (show_all || show_listening) && show_tcp {
            out.push_str("tcp    LISTEN     0      128    0.0.0.0:22             0.0.0.0:*\n");
            out.push_str("tcp    LISTEN     0      128    0.0.0.0:80             0.0.0.0:*\n");
        }

        for socket_line in self.network.list_sockets() {
            out.push_str("tcp    ");
            out.push_str(&socket_line);
            out.push('\n');
        }

        let _ = show_numeric;
        out
    }

    fn cmd_ping(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: ping <host>".to_string();
        }

        // Get the host (last non-flag argument)
        let host = args
            .iter()
            .filter(|a| !a.starts_with('-'))
            .next_back()
            .unwrap_or(&"");

        if host.is_empty() {
            return "ping: missing host operand".to_string();
        }

        // Return escape sequence for real ping
        format!("\x1b[PING:{}]", host)
    }

    fn cmd_host(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: host <hostname>".to_string();
        }

        let hostname = args.last().unwrap_or(&"");

        // Return escape sequence for real DNS lookup
        format!("\x1b[DNS:{}]", hostname)
    }

    fn cmd_myip(&self) -> String {
        "\x1b[MYIP]".to_string()
    }

    fn cmd_traceroute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: traceroute <host>".to_string();
        }

        let host = args.last().unwrap_or(&"");
        let hops = self.network.traceroute_hops(host);

        let mut out = format!(
            "traceroute to {} ({}), 30 hops max, 60 byte packets\n",
            host,
            hops.last()
                .map(|(_, ip, _)| ip.as_str())
                .unwrap_or("0.0.0.0")
        );

        for (hop, ip, time) in hops {
            out.push_str(&format!(
                " {:2}  {}  {:.3} ms  {:.3} ms  {:.3} ms\n",
                hop,
                ip,
                time,
                time + 0.1,
                time + 0.2
            ));
        }

        out
    }

    fn cmd_ifconfig(&self, args: &[&str]) -> String {
        let interfaces = self.network.get_interfaces();
        let filter = args.first().copied();

        let mut out = String::new();
        for iface in &interfaces {
            if let Some(name) = filter {
                if iface.name != name {
                    continue;
                }
            }

            let flags = if iface.is_up { "UP" } else { "DOWN" };
            let loopback = if iface.is_loopback { ",LOOPBACK" } else { "" };

            out.push_str(&format!(
                "{}: flags=4163<{}{},BROADCAST,RUNNING,MULTICAST>  mtu {}\n",
                iface.name, flags, loopback, iface.mtu
            ));
            out.push_str(&format!(
                "        inet {}  netmask 255.255.255.0  broadcast 192.168.1.255\n",
                iface.ipv4
            ));
            out.push_str(&format!(
                "        inet6 {}  prefixlen 64  scopeid 0x20<link>\n",
                iface.ipv6
            ));
            out.push_str(&format!(
                "        ether {}  txqueuelen 1000  (Ethernet)\n",
                iface.mac
            ));
            out.push_str(&format!(
                "        RX packets {}  bytes {} ({:.1} KB)\n",
                iface.rx_packets,
                iface.rx_bytes,
                iface.rx_bytes as f64 / 1024.0
            ));
            out.push_str(&format!(
                "        TX packets {}  bytes {} ({:.1} KB)\n\n",
                iface.tx_packets,
                iface.tx_bytes,
                iface.tx_bytes as f64 / 1024.0
            ));
        }

        if out.is_empty() && filter.is_some() {
            format!(
                "{}: error fetching interface information: Device not found",
                filter.unwrap()
            )
        } else {
            out
        }
    }

    fn cmd_ip(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: ip [ OPTIONS ] OBJECT { COMMAND }\n       ip addr\n       ip link\n       ip route\n       ip neigh".to_string();
        }

        match args[0] {
            "addr" | "a" | "address" => {
                let mut out = String::new();
                for (i, iface) in self.network.get_interfaces().iter().enumerate() {
                    let state = if iface.is_up { "UP" } else { "DOWN" };
                    out.push_str(&format!(
                        "{}: {}: <BROADCAST,MULTICAST,{}> mtu {} state {}\n",
                        i + 1,
                        iface.name,
                        state,
                        iface.mtu,
                        state
                    ));
                    out.push_str(&format!(
                        "    link/ether {} brd ff:ff:ff:ff:ff:ff\n",
                        iface.mac
                    ));
                    out.push_str(&format!(
                        "    inet {}/24 brd 192.168.1.255 scope global {}\n",
                        iface.ipv4, iface.name
                    ));
                    out.push_str(&format!("    inet6 {}/64 scope link\n", iface.ipv6));
                }
                out
            }
            "link" | "l" => {
                let mut out = String::new();
                for (i, iface) in self.network.get_interfaces().iter().enumerate() {
                    let state = if iface.is_up { "UP" } else { "DOWN" };
                    out.push_str(&format!(
                        "{}: {}: <BROADCAST,MULTICAST,{}> mtu {} state {}\n",
                        i + 1,
                        iface.name,
                        state,
                        iface.mtu,
                        state
                    ));
                    out.push_str(&format!(
                        "    link/ether {} brd ff:ff:ff:ff:ff:ff\n",
                        iface.mac
                    ));
                }
                out
            }
            "route" | "r" => self.cmd_route(&[]),
            "neigh" | "neighbor" => self.cmd_arp(&[]),
            _ => format!("ip: unknown command '{}'", args[0]),
        }
    }

    fn cmd_route(&self, args: &[&str]) -> String {
        if args.first() == Some(&"-n") || args.is_empty() {
            let routes = self.network.get_routes();
            let mut out = String::from("Kernel IP routing table\n");
            out.push_str(
                "Destination     Gateway         Genmask         Flags Metric Ref    Use Iface\n",
            );
            for route in routes {
                out.push_str(&format!(
                    "{:<15} {:<15} {:<15} {:<5} {:>6} {:>3} {:>6} {}\n",
                    route.destination,
                    route.gateway,
                    route.genmask,
                    route.flags,
                    0,
                    0,
                    0,
                    route.iface
                ));
            }
            out
        } else {
            "route: unknown option".to_string()
        }
    }

    fn cmd_arp(&self, args: &[&str]) -> String {
        let show_numeric = args.contains(&"-n");
        let arp_entries = self.network.arp_table();

        let mut out = String::from(
            "Address                  HWtype  HWaddress           Flags Mask  Iface\n",
        );
        for (ip, mac, iface) in arp_entries {
            out.push_str(&format!(
                "{:<24} ether   {:<17} C           {}\n",
                ip, mac, iface
            ));
        }

        let _ = show_numeric;
        out
    }

    fn cmd_nc(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: nc [-lvnz] hostname port".to_string();
        }

        let listen_mode = args.contains(&"-l");
        let verbose = args.contains(&"-v");
        let scan_mode = args.contains(&"-z");

        // Get host and port from non-flag args
        let positional: Vec<&str> = args
            .iter()
            .filter(|a| !a.starts_with('-'))
            .copied()
            .collect();

        if listen_mode {
            let port = positional.first().unwrap_or(&"0");
            format!("Listening on 0.0.0.0 {}", port)
        } else if positional.len() < 2 {
            "nc: missing hostname and port".to_string()
        } else {
            let host = positional[0];
            let port = positional[1];

            if scan_mode {
                format!("Connection to {} {} port [tcp/*] succeeded!", host, port)
            } else if verbose {
                format!(
                    "Connection to {} {} port [tcp/*] succeeded!\n[Connected - type to send data]",
                    host, port
                )
            } else {
                format!("[Connected to {}:{}]", host, port)
            }
        }
    }

    fn cmd_socket(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: socket <ws|http> <action> [args...]".to_string();
        }

        let protocol = match args[0].to_lowercase().as_str() {
            "ws" | "websocket" => Protocol::WebSocket,
            "http" => Protocol::Http,
            "tcp" => Protocol::Tcp,
            "udp" => Protocol::Udp,
            _ => return format!("socket: unknown protocol '{}'", args[0]),
        };

        if args.len() < 2 {
            return "usage: socket <proto> <action> [args...]".to_string();
        }

        match args[1] {
            "create" => {
                let id = self.network.socket(protocol);
                format!("Created socket {}", id)
            }
            "connect" => {
                if args.len() < 3 {
                    return "usage: socket <proto> connect <url>".to_string();
                }
                let id = self.network.socket(protocol);
                let url = args[2];
                match self.network.connect_ws(id, url) {
                    Ok(()) => format!("Connecting socket {} to {}", id, url),
                    Err(e) => format!("Error: {}", e),
                }
            }
            "send" => {
                if args.len() < 4 {
                    return "usage: socket <proto> send <socket_id> <data>".to_string();
                }
                let id: u32 = args[2].parse().unwrap_or(0);
                let data = args[3..].join(" ");
                match self.network.send(id, &data) {
                    Ok(()) => format!("Sent {} bytes", data.len()),
                    Err(e) => format!("Error: {}", e),
                }
            }
            "close" => {
                if args.len() < 3 {
                    return "usage: socket <proto> close <socket_id>".to_string();
                }
                let id: u32 = args[2].parse().unwrap_or(0);
                match self.network.close(id) {
                    Ok(()) => format!("Closed socket {}", id),
                    Err(e) => format!("Error: {}", e),
                }
            }
            _ => format!("socket: unknown action '{}'", args[1]),
        }
    }

    fn cmd_service(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return self.services.list().join("\n");
        }

        match args[0] {
            "list" => self.services.list().join("\n"),
            "start" => {
                if args.len() < 2 {
                    return "usage: service start <name>".to_string();
                }
                let name = args[1];
                let pid = self.kernel.proc.spawn(name, 1);
                match self.services.start(name, pid) {
                    Ok(()) => format!("Started service '{}'", name),
                    Err(e) => format!("Error: {}", e),
                }
            }
            "stop" => {
                if args.len() < 2 {
                    return "usage: service stop <name>".to_string();
                }
                let name = args[1];
                match self.services.stop(name) {
                    Ok(()) => format!("Stopped service '{}'", name),
                    Err(e) => format!("Error: {}", e),
                }
            }
            "restart" => {
                if args.len() < 2 {
                    return "usage: service restart <name>".to_string();
                }
                let name = args[1];
                let pid = self.kernel.proc.spawn(name, 1);
                match self.services.restart(name, pid) {
                    Ok(()) => format!("Restarted service '{}'", name),
                    Err(e) => format!("Error: {}", e),
                }
            }
            "status" => {
                if args.len() < 2 {
                    return "usage: service status <name>".to_string();
                }
                let name = args[1];
                match self.services.get_state(name) {
                    Some(state) => format!("{}: {:?}", name, state),
                    None => format!("Service '{}' not found", name),
                }
            }
            _ => format!("service: unknown action '{}'", args[0]),
        }
    }

    // Removed unused cmd_neofetch (handled inline in exec match) to silence warning.

    fn start_python_repl(&mut self) -> String {
        self.python_interp = Some(PythonInterpreter::new());
        self.in_python_repl = true;
        "\x1b[PYTHON_REPL]".into()
    }

    #[wasm_bindgen]
    pub fn exec_python(&mut self, code: &str) -> String {
        if code.trim() == "exit()" {
            self.in_python_repl = false;
            self.python_interp = None;
            return "\x1b[EXIT_PYTHON]".to_string();
        }

        if let Some(ref mut interp) = self.python_interp {
            match interp.eval(code) {
                Ok(result) => result,
                Err(e) => format!("Error: {}", e),
            }
        } else {
            "Error: Python interpreter not initialized".to_string()
        }
    }

    #[wasm_bindgen]
    pub fn is_in_python_repl(&self) -> bool {
        self.in_python_repl
    }

    // Syscalls
    #[wasm_bindgen]
    pub fn sys_open(&mut self, path: &str, write: bool) -> i32 {
        self.kernel
            .fs
            .open(path, write)
            .map(|h| h as i32)
            .unwrap_or(-1)
    }
    #[wasm_bindgen]
    pub fn sys_read(&mut self, handle: u32, size: u32) -> String {
        self.kernel
            .fs
            .read(handle, size as usize)
            .unwrap_or_default()
    }
    #[wasm_bindgen]
    pub fn sys_write(&mut self, handle: u32, data: &str) -> bool {
        self.kernel.fs.write(handle, data).is_ok()
    }
    #[wasm_bindgen]
    pub fn sys_close(&mut self, handle: u32) {
        self.kernel.fs.close(handle);
    }
    #[wasm_bindgen]
    pub fn complete(&self, partial: &str) -> Vec<JsValue> {
        let mut matches = Vec::new();
        let cmds = [
            "cat", "cd", "clear", "echo", "env", "exit", "export", "free", "help", "history",
            "hostname", "id", "kill", "ls", "mkdir", "ps", "pwd", "rm", "touch", "uname", "uptime",
            "whoami",
        ];
        for c in cmds {
            if c.starts_with(partial) {
                matches.push(JsValue::from_str(c));
            }
        }
        if let Some(dir) = self.kernel.fs.resolve(&self.kernel.fs.cwd) {
            for name in dir.children.keys() {
                if name.starts_with(partial) {
                    matches.push(JsValue::from_str(name));
                }
            }
        }
        matches
    }
}
