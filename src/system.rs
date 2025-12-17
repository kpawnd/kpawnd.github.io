use crate::{
    boot::BootManager,
    kernel::Kernel,
    network::{NetworkStack, Protocol},
    process::{Priority, ProcState},
    python::PythonInterpreter,
    services::ServiceManager,
    shell::{prompt, Shell},
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct System {
    boot: BootManager,
    kernel: Kernel,
    shell: Shell,
    network: NetworkStack,
    services: ServiceManager,
    booted: bool,
    cleared_after_boot: bool,
    python_interp: Option<PythonInterpreter>,
    in_python_repl: bool,
    user_password: Option<String>,
    sudo_pending_cmd: Option<String>,
    sudo_waiting_password: bool,
    sudo_authenticated_until: Option<f64>,
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
            boot: BootManager::new(),
            kernel: Kernel::new(),
            shell: Shell::new(),
            network: NetworkStack::new(),
            services: ServiceManager::new(),
            booted: false,
            cleared_after_boot: false,
            python_interp: None,
            in_python_repl: false,
            user_password: None,
            sudo_pending_cmd: None,
            sudo_waiting_password: false,
            sudo_authenticated_until: None,
        };

        // Auto-start system services
        system
            .services
            .auto_start_services(&mut |name| system.kernel.proc.spawn(name, 1, &mut system.kernel.mem));

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
        let user = self
            .shell
            .env
            .get("USER")
            .cloned()
            .unwrap_or_else(|| "user".into());
        let home = self
            .shell
            .env
            .get("HOME")
            .cloned()
            .unwrap_or_else(|| "/home/user".into());
        prompt(&self.kernel, &user, &home)
    }

    #[wasm_bindgen]
    pub fn exec(&mut self, line: &str) -> String {
        self.kernel.tick();
        self.kernel.scheduler.tick(&mut self.kernel.proc);
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            self.shell.history.push(trimmed.into());
        }
        if self.sudo_waiting_password {
            let cmd = self.sudo_pending_cmd.take().unwrap_or_default();
            self.sudo_waiting_password = false;
            return self.exec_sudo(&cmd, trimmed);
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return String::new();
        }
        let cmd = parts[0];
        let args = &parts[1..];
        if cmd == "sudo" {
            if args.is_empty() {
                return "usage: sudo <command>".into();
            }
            // Check if we have a valid cached sudo session (5 minute timeout)
            let now = js_sys::Date::now();
            let is_authenticated = self
                .sudo_authenticated_until
                .map(|until| now < until)
                .unwrap_or(false);

            if is_authenticated {
                // Execute directly without password prompt
                return self.exec_sudo_internal(&args.join(" "));
            } else {
                // Need password
                self.sudo_pending_cmd = Some(args.join(" "));
                self.sudo_waiting_password = true;
                return format!(
                    "[sudo] password for {}:",
                    self.shell.env.get("USER").unwrap_or(&"user".to_string())
                );
            }
        }
        if self.shell.registry.has(cmd) {
            if let Some(pid) = self.kernel.proc.spawn(cmd, 1, &mut self.kernel.mem) {
                self.kernel.scheduler.add(pid, Priority::Normal);
            } else {
                return "Failed to spawn process: out of memory".to_string();
            }
        }
        match cmd {
            "reboot" => "\x1b[REBOOT]".into(),
            "echo" => { let out=args.join(" "); if out=="github" { format!("\x1b[OPEN:{}]", self.shell.env.get("GITHUB").unwrap()) } else { out } }
            "help" => "Available commands:\n\n  File operations:    cat cd chmod chown cp cut diff du file find head ln ls mkdir mv pwd rm rmdir sort tail tee touch tr uniq wc nano vi\n  Text processing:    awk grep sed\n  System info:        df free hostname id man neofetch ps top uname uptime whereis which whoami\n  Network:            arp curl dig host ifconfig ip myip nc netstat nslookup\n                      ping route ss traceroute wget\n  Archives:           tar gzip gunzip zip unzip\n  Package mgmt:       apt apt-get\n  Games:              doom doommap mp\n  Other:              alias clear echo env exit export grub hasgrub help history kill\n                      python screensaver service sudo\n\nType 'man <command>' for more info on a specific command.".into(),
            "man" => self.cmd_man(&args),
            "neofetch" => "\x1b[NEOFETCH_DATA]".to_string(),
            "nano" | "vi" | "vim" => self.cmd_nano(&args),
            "python" => { if args.is_empty() { self.start_python_repl() } else { "python: script execution not supported".to_string() } }
            "doom" => {
                // Parse optional difficulty argument: easy|normal|hard or 0|1|2
                if !args.is_empty() {
                    let raw = args[0].to_lowercase();
                    let diff = match raw.as_str() {
                        "easy" | "0" => Some(0u8),
                        "normal" | "1" => Some(1u8),
                        "hard" | "2" => Some(2u8),
                        _ => None,
                    };
                    if let Some(d) = diff {
                        return format!("\x1b[LAUNCH_DOOM:{}]", d);
                    } else {
                        return "usage: doom [easy|normal|hard]".to_string();
                    }
                }
                "\x1b[LAUNCH_DOOM]".to_string()
            },
            "doommap" => {
                if args.is_empty() {
                    return "usage: doommap <proc|restore>".into();
                }
                match args[0] {
                    "proc" => "\x1b[DOOM_ENABLE_PROC]".into(),
                    "restore" => "\x1b[DOOM_RESTORE]".into(),
                    _ => "usage: doommap <proc|restore>".into(),
                }
            },
            "grace" => {
                // Launch the desktop environment named Grace
                "\x1b[LAUNCH_GRACE]".into()
            },
            "mp" => {
                if args.is_empty() { return "usage: mp <host|join|finalize|id|disconnect>".into(); }
                match args[0] {
                    "host" => "\x1b[MP_HOST]".into(),
                    "join" => {
                        if args.len()<2 { "usage: mp join <ROOM_CODE>".into() } else { format!("\x1b[MP_JOIN:{}]", args[1]) }
                    }
                    "finalize" => {
                        if args.len()<2 { "usage: mp finalize <ANSWER_CODE>".into() } else { format!("\x1b[MP_FINALIZE:{}]", args[1]) }
                    }
                    "id" => "\x1b[MP_ID]".into(),
                    "disconnect" => "\x1b[MP_DISCONNECT]".into(),
                    _ => "usage: mp <host|join|finalize|id|disconnect>".into(),
                }
            },
            "screensaver" => "\x1b[LAUNCH_SCREENSAVER]".to_string(),
            "wget" => self.cmd_wget(&args),
            "curl" => self.cmd_curl(&args),
            "myip" => self.cmd_myip(),
            "ls" => self.cmd_ls(&args),
            "cd" => self.cmd_cd(&args),
            "pwd" => self.kernel.fs.cwd.clone(),
            "cat" => self.cmd_cat(&args),
            "grep" => self.cmd_grep(&args),
            "find" => self.cmd_find(&args),
            "wc" => self.cmd_wc(&args),
            "head" => self.cmd_head(&args),
            "tail" => self.cmd_tail(&args),
            "diff" => self.cmd_diff(&args),
            "sort" => self.cmd_sort(&args),
            "uniq" => self.cmd_uniq(&args),
            "cut" => self.cmd_cut(&args),
            "tr" => self.cmd_tr(&args),
            "tee" => self.cmd_tee(&args),
            "which" => self.cmd_which(&args),
            "whereis" => self.cmd_whereis(&args),
            "file" => self.cmd_file(&args),
            "ln" => self.cmd_ln(&args),
            "cp" => self.cmd_cp(&args),
            "mv" => self.cmd_mv(&args),
            "chmod" => self.cmd_chmod(&args),
            "chown" => self.cmd_chown(&args),
            "df" => self.cmd_df(&args),
            "du" => self.cmd_du(&args),
            "tar" => self.cmd_tar(&args),
            "gzip" | "gunzip" => self.cmd_gzip(&args, cmd),
            "zip" | "unzip" => self.cmd_zip(&args, cmd),
            "apt" | "apt-get" => self.cmd_apt(&args),
            "top" => self.cmd_top(),
            "awk" => self.cmd_awk(&args),
            "sed" => self.cmd_sed(&args),
            "alias" => self.cmd_alias(&args),
            "touch" => self.cmd_touch(&args),
            "mkdir" => self.cmd_mkdir(&args),
            "rm" => self.cmd_rm(&args),
            "clear" => "\x1b[CLEAR]".into(),
            "exit" => "\x1b[EXIT]".into(),
            "ps" => self.cmd_ps(),
            "kill" => self.cmd_kill(&args),
            "uname" => self.cmd_uname(&args),
            "hostname" => self.cmd_hostname(),
            "id" => {
                let user = self
                    .shell
                    .env
                    .get("USER")
                    .cloned()
                    .unwrap_or_else(|| "user".into());
                format!("uid=1000({}) gid=1000({})", user, user)
            }
            "whoami" => self
                .shell
                .env
                .get("USER")
                .cloned()
                .unwrap_or_else(|| "user".into()),
            "uptime" => format!("up {}ms", self.kernel.uptime_ms()),
            "free" => self.cmd_free(),
            "history" => self.cmd_history(),
            "env" => self.cmd_env(),
            "export" => self.cmd_export(&args),
            "netstat" => self.cmd_netstat(&args),
            "ss" => self.cmd_ss(&args),
            "socket" => self.cmd_socket(&args),
            "service" => self.cmd_service(&args),
            "ping" => self.cmd_ping(&args),
            "traceroute" | "tracert" => self.cmd_traceroute(&args),
            "ifconfig" => self.cmd_ifconfig(&args),
            "ip" => self.cmd_ip(&args),
            "route" => self.cmd_route(&args),
            "arp" => self.cmd_arp(&args),
            "host" | "nslookup" | "dig" => self.cmd_host(&args),
            "nc" | "netcat" => self.cmd_nc(&args),
            "hasgrub" => if self.has_grub() { "yes".into() } else { "no".into() },
            "grub" => {
                if args.is_empty() {
                    return "usage: grub <switch|status|boot>".into();
                }
                match args[0] {
                    "switch" => {
                        if args.len() < 2 {
                            return "usage: grub switch <bootloader>".into();
                        }
                        match self.boot.set_bootloader(&args[1]) {
                            Ok(_) => format!("Switched to {} bootloader", args[1]),
                            Err(e) => format!("Error: {}", e),
                        }
                    }
                    "status" => {
                        let current = self.boot.get_current_bootloader();
                        let available = self.boot.list_bootloaders().join(", ");
                        format!("Current bootloader: {}\nAvailable bootloaders: {}", current, available)
                    }
                    "boot" => {
                        let messages = self.boot.simulate_boot_sequence(&mut self.kernel.mem);
                        format!("\x1b[BOOT_SEQUENCE:{}]", messages.join("|"))
                    }
                    _ => "usage: grub <switch|status|boot>".into(),
                }
            }
            "" => String::new(),
            _ => format!("sh: {}: command not found", cmd),
        }
    }

    #[wasm_bindgen]
    pub fn set_user_password(&mut self, pw: &str) {
        self.user_password = Some(pw.into());
    }

    fn exec_sudo_internal(&mut self, cmd: &str) -> String {
        let old_user = self
            .shell
            .env
            .get("USER")
            .cloned()
            .unwrap_or_else(|| "user".into());
        let old_home = self
            .shell
            .env
            .get("HOME")
            .cloned()
            .unwrap_or_else(|| "/home/user".into());
        let old_owner = self.kernel.fs.get_default_owner();
        let old_group = self.kernel.fs.get_default_group();

        self.shell.env.insert("USER".into(), "root".into());
        self.shell.env.insert("HOME".into(), "/root".into());
        let _ = self.kernel.fs.create_dir("/root");
        self.kernel.fs.set_default_owner("root", "root");

        let out = self.exec(cmd);

        // revert
        self.shell.env.insert("USER".into(), old_user);
        self.shell.env.insert("HOME".into(), old_home);
        self.kernel.fs.set_default_owner(&old_owner, &old_group);
        out
    }

    #[wasm_bindgen]
    pub fn exec_sudo(&mut self, cmd: &str, pw: &str) -> String {
        match &self.user_password {
            Some(saved) if saved == pw => {
                // Set sudo session to expire in 5 minutes
                let now = js_sys::Date::now();
                self.sudo_authenticated_until = Some(now + 300000.0);
                self.exec_sudo_internal(cmd)
            }
            _ => "sudo: incorrect password".into(),
        }
    }

    #[wasm_bindgen]
    pub fn is_waiting_for_sudo(&self) -> bool {
        self.sudo_waiting_password
    }

    #[wasm_bindgen]
    pub fn has_grub(&self) -> bool {
        // Ensure filesystem initialized before checking
        if self.kernel.fs.resolve("/boot").is_none() {
            // has_grub is a quick probe
            // Workaround by temporarily casting
            let this = self as *const System as *mut System;
            unsafe {
                (*this).kernel.fs.init();
            }
        }
        self.kernel.fs.resolve("/boot/grub/grub.cfg").is_some()
    }

    fn cmd_ls(&self, args: &[&str]) -> String {
        let mut show_all = false;
        let mut show_long = false;
        let mut path = ".";

        for arg in args {
            if *arg == "-l" {
                show_long = true;
            } else if *arg == "-a" {
                show_all = true;
            } else if *arg == "-la" || *arg == "-al" {
                show_long = true;
                show_all = true;
            } else if !arg.starts_with('-') {
                path = arg;
            }
        }

        match self.kernel.fs.resolve(path) {
            Some(node) if node.is_dir => {
                let mut entries: Vec<_> = node.children.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));

                if show_long {
                    let mut out = String::new();
                    if show_all {
                        out.push_str("drwxr-xr-x   2 user     user         4096 Nov 29 12:00 \x1b[COLOR:blue].\x1b[COLOR:reset]\n");
                        out.push_str("drwxr-xr-x   2 root     root         4096 Nov 29 12:00 \x1b[COLOR:blue]..\x1b[COLOR:reset]\n");
                    }
                    for (name, child) in &entries {
                        if !show_all && name.starts_with('.') {
                            continue;
                        }
                        let name_display = if child.is_dir {
                            format!("\x1b[COLOR:blue]{}\x1b[COLOR:reset]", name)
                        } else if child.is_executable {
                            format!("\x1b[COLOR:green]{}\x1b[COLOR:reset]", name)
                        } else {
                            name.to_string()
                        };
                        out.push_str(&format!(
                            "{} {:>3} {:>8} {:>8} {:>8} {} {}\n",
                            child.permissions,
                            1,
                            child.owner,
                            child.group,
                            child.size,
                            "Nov 29 12:00",
                            name_display
                        ));
                    }
                    out.trim_end().to_string()
                } else {
                    let names: Vec<String> = entries
                        .iter()
                        .filter(|(name, _)| show_all || !name.starts_with('.'))
                        .map(|(name, child)| {
                            if child.is_dir {
                                format!("\x1b[COLOR:blue]{}\x1b[COLOR:reset]", name)
                            } else if child.is_executable {
                                format!("\x1b[COLOR:green]{}\x1b[COLOR:reset]", name)
                            } else {
                                name.to_string()
                            }
                        })
                        .collect();
                    names.join("  ")
                }
            }
            Some(node) => node.name.clone(),
            None => format!("ls: cannot access '{}': No such file or directory", path),
        }
    }
    fn cmd_cd(&mut self, args: &[&str]) -> String {
        let default_home = self
            .shell
            .env
            .get("HOME")
            .cloned()
            .unwrap_or_else(|| "/home/user".into());
        let target = if args.is_empty() {
            default_home.as_str()
        } else {
            args[0]
        };
        match self.kernel.fs.cd(target) {
            Ok(()) => String::new(),
            Err(e) => format!("cd: {}: {}", target, e),
        }
    }
    fn cmd_cat(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "cat: missing operand".into();
        }
        match self.kernel.fs.resolve(args[0]) {
            Some(n) if !n.is_dir => n.data.clone(),
            Some(_) => format!("cat: {}: Is a directory", args[0]),
            None => format!("cat: {}: No such file or directory", args[0]),
        }
    }

    #[wasm_bindgen]
    pub fn set_user(&mut self, username: &str) {
        let uname = if username.is_empty() {
            "user"
        } else {
            username
        };
        self.shell.env.insert("USER".into(), uname.into());
        let home = format!("/home/{}", uname);
        self.shell.env.insert("HOME".into(), home.clone());
        // Ensure home directory exists
        let _ = self.kernel.fs.create_dir(&home);
        // Update default owner for new files/directories
        self.kernel.fs.set_default_owner(uname, uname);
    }
    fn cmd_touch(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "touch: missing file operand".into();
        }
        match self.kernel.fs.create_file(args[0], "") {
            Ok(()) => String::new(),
            Err(e) => format!("touch: cannot touch '{}': {}", args[0], e),
        }
    }
    fn cmd_mkdir(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "mkdir: missing operand".into();
        }
        match self.kernel.fs.create_dir(args[0]) {
            Ok(()) => String::new(),
            Err(e) => format!("mkdir: cannot create directory '{}': {}", args[0], e),
        }
    }
    fn cmd_rm(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "rm: missing operand".into();
        }

        let mut force = false;
        let mut recursive = false;
        let mut files = Vec::new();

        for arg in args {
            match *arg {
                "-f" | "--force" => force = true,
                "-r" | "-rf" | "-fr" => {
                    recursive = true;
                    force = true;
                }
                other => files.push(other),
            }
        }

        if files.is_empty() {
            return "rm: missing operand".into();
        }

        for file in files {
            if recursive {
                self.kernel.fs.set_ignore_critical_deletes(true);
            }
            let res = if recursive {
                self.kernel.fs.remove_recursive(file)
            } else {
                self.kernel.fs.remove(file)
            };
            if recursive {
                self.kernel.fs.set_ignore_critical_deletes(false);
            }
            match res {
                Ok(()) => {}
                Err(e) => {
                    if self.kernel.fs.kernel_panic {
                        return format!("\x1b[KERNEL_PANIC]{}", self.kernel.fs.panic_reason);
                    }
                    if self.kernel.memory_panic {
                        return format!("\x1b[KERNEL_PANIC]{}", self.kernel.memory_panic_reason);
                    }
                    if !force {
                        return format!("rm: cannot remove '{}': {}", file, e);
                    }
                }
            }
        }
        String::new()
    }

    fn cmd_grep(&self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "usage: grep [pattern] [file]".into();
        }
        let pattern = args[0];
        let file_path = args[1];
        match self.kernel.fs.resolve(file_path) {
            Some(node) if !node.is_dir => {
                let lines: Vec<&str> = node.data.lines().collect();
                let matches: Vec<String> = lines
                    .iter()
                    .filter(|line| line.contains(pattern))
                    .map(|s| s.to_string())
                    .collect();
                if matches.is_empty() {
                    String::new()
                } else {
                    matches.join("\n")
                }
            }
            Some(_) => format!("grep: {}: Is a directory", file_path),
            None => format!("grep: {}: No such file or directory", file_path),
        }
    }

    fn cmd_find(&self, args: &[&str]) -> String {
        let path = if args.is_empty() { "." } else { args[0] };
        let mut results = Vec::new();
        self.find_recursive(&self.kernel.fs.normalize(path), &mut results);
        results.join("\n")
    }

    fn find_recursive(&self, path: &str, results: &mut Vec<String>) {
        if let Some(node) = self.kernel.fs.resolve(path) {
            results.push(path.to_string());
            if node.is_dir {
                for name in node.children.keys() {
                    let child_path = if path == "/" {
                        format!("/{}", name)
                    } else {
                        format!("{}/{}", path, name)
                    };
                    self.find_recursive(&child_path, results);
                }
            }
        }
    }

    fn cmd_wc(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: wc [file]".into();
        }
        match self.kernel.fs.resolve(args[0]) {
            Some(node) if !node.is_dir => {
                let lines = node.data.lines().count();
                let words = node.data.split_whitespace().count();
                let chars = node.data.len();
                format!("{:7} {:7} {:7} {}", lines, words, chars, args[0])
            }
            Some(_) => format!("wc: {}: Is a directory", args[0]),
            None => format!("wc: {}: No such file or directory", args[0]),
        }
    }

    fn cmd_head(&self, args: &[&str]) -> String {
        let (n, file) = if args.len() >= 2 && args[0] == "-n" {
            (args[1].parse().unwrap_or(10), args.get(2).copied())
        } else {
            (10, args.first().copied())
        };

        if file.is_none() {
            return "usage: head [-n lines] [file]".into();
        }

        match self.kernel.fs.resolve(file.unwrap()) {
            Some(node) if !node.is_dir => node.data.lines().take(n).collect::<Vec<_>>().join("\n"),
            Some(_) => format!("head: {}: Is a directory", file.unwrap()),
            None => format!("head: {}: No such file or directory", file.unwrap()),
        }
    }

    fn cmd_tail(&self, args: &[&str]) -> String {
        let (n, file) = if args.len() >= 2 && args[0] == "-n" {
            (args[1].parse().unwrap_or(10), args.get(2).copied())
        } else {
            (10, args.first().copied())
        };

        if file.is_none() {
            return "usage: tail [-n lines] [file]".into();
        }

        match self.kernel.fs.resolve(file.unwrap()) {
            Some(node) if !node.is_dir => {
                let lines: Vec<&str> = node.data.lines().collect();
                let start = if lines.len() > n { lines.len() - n } else { 0 };
                lines[start..].join("\n")
            }
            Some(_) => format!("tail: {}: Is a directory", file.unwrap()),
            None => format!("tail: {}: No such file or directory", file.unwrap()),
        }
    }

    fn cmd_diff(&self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "usage: diff [file1] [file2]".into();
        }
        let file1 = self.kernel.fs.resolve(args[0]);
        let file2 = self.kernel.fs.resolve(args[1]);

        match (file1, file2) {
            (Some(f1), Some(f2)) if !f1.is_dir && !f2.is_dir => {
                if f1.data == f2.data {
                    String::new()
                } else {
                    format!("Files {} and {} differ", args[0], args[1])
                }
            }
            _ => "diff: invalid files".into(),
        }
    }

    fn cmd_sort(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: sort [file]".into();
        }
        match self.kernel.fs.resolve(args[0]) {
            Some(node) if !node.is_dir => {
                let mut lines: Vec<&str> = node.data.lines().collect();
                lines.sort();
                lines.join("\n")
            }
            Some(_) => format!("sort: {}: Is a directory", args[0]),
            None => format!("sort: {}: No such file or directory", args[0]),
        }
    }

    fn cmd_uniq(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: uniq [file]".into();
        }
        match self.kernel.fs.resolve(args[0]) {
            Some(node) if !node.is_dir => {
                let lines: Vec<&str> = node.data.lines().collect();
                let mut result = Vec::new();
                let mut last = "";
                for line in lines {
                    if line != last {
                        result.push(line);
                        last = line;
                    }
                }
                result.join("\n")
            }
            Some(_) => format!("uniq: {}: Is a directory", args[0]),
            None => format!("uniq: {}: No such file or directory", args[0]),
        }
    }

    fn cmd_cut(&self, _args: &[&str]) -> String {
        "cut: simplified implementation not available".into()
    }

    fn cmd_tr(&self, _args: &[&str]) -> String {
        "tr: simplified implementation not available".into()
    }

    fn cmd_tee(&self, _args: &[&str]) -> String {
        "tee: simplified implementation not available (no pipe support yet)".into()
    }

    fn cmd_which(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: which [command]".into();
        }
        let cmd = args[0];
        if self.shell.registry.has(cmd) || self.is_builtin(cmd) {
            format!("/usr/bin/{}", cmd)
        } else {
            format!("which: no {} in (/usr/bin:/bin:/usr/sbin:/sbin)", cmd)
        }
    }

    fn cmd_whereis(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: whereis [command]".into();
        }
        let cmd = args[0];
        if self.shell.registry.has(cmd) || self.is_builtin(cmd) {
            format!("{}: /usr/bin/{} /usr/share/man/man1/{}.1.gz", cmd, cmd, cmd)
        } else {
            format!("{}: not found", cmd)
        }
    }

    fn cmd_file(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: file [file]".into();
        }
        match self.kernel.fs.resolve(args[0]) {
            Some(node) if node.is_dir => format!("{}: directory", args[0]),
            Some(node) if node.is_executable => {
                format!("{}: ELF 64-bit LSB executable, x86-64", args[0])
            }
            Some(node) if node.permissions.starts_with('l') => {
                format!("{}: symbolic link to {}", args[0], node.data)
            }
            Some(node) if node.data.starts_with('#') => format!("{}: ASCII text", args[0]),
            Some(_) => format!("{}: data", args[0]),
            None => format!("{}: cannot open (No such file or directory)", args[0]),
        }
    }

    fn cmd_ln(&mut self, _args: &[&str]) -> String {
        "ln: symbolic links not fully implemented".into()
    }

    fn cmd_cp(&mut self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "usage: cp [source] [dest]".into();
        }
        let data = match self.kernel.fs.resolve(args[0]) {
            Some(node) if !node.is_dir => node.data.clone(),
            Some(_) => return "cp: omitting directory (use -r for recursive)".into(),
            None => return format!("cp: cannot stat '{}': No such file or directory", args[0]),
        };

        match self.kernel.fs.create_file(args[1], &data) {
            Ok(()) => String::new(),
            Err(e) => format!("cp: cannot create '{}': {}", args[1], e),
        }
    }

    fn cmd_mv(&mut self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "usage: mv [source] [dest]".into();
        }
        match self.kernel.fs.resolve(args[0]) {
            Some(node) if !node.is_dir => {
                let data = node.data.clone();
                match self.kernel.fs.create_file(args[1], &data) {
                    Ok(()) => {
                        let _ = self.kernel.fs.remove(args[0]);
                        String::new()
                    }
                    Err(e) => format!("mv: cannot move to '{}': {}", args[1], e),
                }
            }
            Some(_) => "mv: cannot move directory".into(),
            None => format!("mv: cannot stat '{}': No such file or directory", args[0]),
        }
    }

    fn cmd_chmod(&mut self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "usage: chmod [mode] [file]".into();
        }
        "chmod: permissions are simulated (no effect)".into()
    }

    fn cmd_chown(&mut self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "usage: chown [owner] [file]".into();
        }
        "chown: ownership changes are simulated (no effect)".into()
    }

    fn cmd_df(&self, _args: &[&str]) -> String {
        let (used, total) = self.kernel.mem.usage();
        format!(
            "Filesystem     1K-blocks    Used Available Use% Mounted on\n\
             /dev/sda1      {}  {}   {}  {}% /",
            total / 1024,
            used / 1024,
            (total - used) / 1024,
            (used * 100) / total
        )
    }

    fn cmd_du(&self, args: &[&str]) -> String {
        let path = if args.is_empty() { "." } else { args[0] };
        match self.kernel.fs.resolve(path) {
            Some(node) if node.is_dir => {
                let size = Self::calc_dir_size(node);
                format!("{}\t{}", size / 1024, path)
            }
            Some(node) => format!("{}\t{}", node.size / 1024, path),
            None => format!("du: cannot access '{}': No such file or directory", path),
        }
    }

    fn calc_dir_size(node: &crate::vfs::Inode) -> usize {
        let mut total = 4096; // directory itself
        for child in node.children.values() {
            if child.is_dir {
                total += Self::calc_dir_size(child);
            } else {
                total += child.size;
            }
        }
        total
    }

    fn cmd_tar(&self, _args: &[&str]) -> String {
        "tar: archive creation/extraction not implemented".into()
    }

    fn cmd_gzip(&self, _args: &[&str], cmd: &str) -> String {
        if cmd == "gzip" {
            "gzip: compression not implemented".into()
        } else {
            "gunzip: decompression not implemented".into()
        }
    }

    fn cmd_zip(&self, _args: &[&str], cmd: &str) -> String {
        if cmd == "zip" {
            "zip: compression not implemented".into()
        } else {
            "unzip: decompression not implemented".into()
        }
    }

    fn cmd_apt(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: apt [install|remove|update|upgrade|search] [package]".into();
        }
        match args[0] {
            "update" => "Reading package lists... Done\nBuilding dependency tree... Done\nAll packages are up to date.".into(),
            "upgrade" => "Reading package lists... Done\nBuilding dependency tree... Done\n0 upgraded, 0 newly installed, 0 to remove and 0 not upgraded.".into(),
            "install" => {
                if args.len() < 2 {
                    return "usage: apt install [package]".into();
                }
                format!("Reading package lists... Done\nBuilding dependency tree... Done\nThe following NEW packages will be installed:\n  {}\n0 upgraded, 1 newly installed, 0 to remove.\nNeed to get 1024 kB of archives.\nAfter this operation, 4096 kB of additional disk space will be used.\nGet:1 http://archive.ubuntu.com/ubuntu {} [1024 kB]\nFetched 1024 kB in 1s\nSelecting previously unselected package {}.\nPreparing to unpack .../{}_{}_amd64.deb ...\nUnpacking {} ...\nSetting up {} ...", args[1], args[1], args[1], args[1], "1.0.0", args[1], args[1])
            }
            "remove" => {
                if args.len() < 2 {
                    return "usage: apt remove [package]".into();
                }
                format!("Reading package lists... Done\nBuilding dependency tree... Done\nThe following packages will be REMOVED:\n  {}\n0 upgraded, 0 newly installed, 1 to remove.\nAfter this operation, 4096 kB disk space will be freed.\nRemoving {} ...", args[1], args[1])
            }
            "search" => {
                if args.len() < 2 {
                    return "usage: apt search [query]".into();
                }
                "Sorting... Done\nFull Text Search... Done\nvim/stable 8.2.2434-3 amd64\n  Vi IMproved - enhanced vi editor\n\nnano/stable 5.4-2 amd64\n  small, friendly text editor inspired by Pico".to_string()
            }
            _ => format!("E: Invalid operation {}", args[0]),
        }
    }

    fn cmd_top(&self) -> String {
        let total_mem = self.kernel.mem.total;
        let free_mem = self.kernel.mem.free;
        let used_mem = total_mem - free_mem;
        format!(
            "top - {}  up {}ms,  1 user,  load average: 0.00, 0.00, 0.00\n\
             Tasks: {} total,   1 running,   {} sleeping,   0 stopped,   0 zombie\n\
             %Cpu(s):  0.3 us,  0.1 sy,  0.0 ni, 99.6 id,  0.0 wa,  0.0 hi,  0.0 si,  0.0 st\n\
             MiB Mem :   {}.0 total,   {}.0 free,   {}.0 used,   {}.0 buff/cache\n\n\
             PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND\n",
            "12:00:00",
            self.kernel.uptime_ms(),
            self.kernel.proc.list().len(),
            self.kernel.proc.list().len() - 1,
            total_mem / 1024 / 1024,
            free_mem / 1024 / 1024,
            used_mem / 1024 / 1024,
            0
        )
    }

    fn cmd_awk(&self, _args: &[&str]) -> String {
        "awk: text processing not fully implemented".into()
    }

    fn cmd_sed(&self, _args: &[&str]) -> String {
        "sed: stream editor not fully implemented".into()
    }

    fn cmd_alias(&self, args: &[&str]) -> String {
        if args.is_empty() {
            "alias ls='ls --color=auto'\nalias ll='ls -la'\nalias la='ls -A'\nalias l='ls -CF'"
                .into()
        } else {
            "alias: dynamic alias creation not implemented".into()
        }
    }

    fn is_builtin(&self, cmd: &str) -> bool {
        matches!(
            cmd,
            "cd" | "exit" | "export" | "pwd" | "echo" | "help" | "history" | "alias"
        )
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
                if self.kernel.proc.kill(pid, &mut self.kernel.mem) {
                    String::new()
                } else {
                    format!("kill: {}: no such process or cannot kill", pid)
                }
            }
            Err(_) => "kill: invalid pid".into(),
        }
    }
    fn cmd_uname(&self, args: &[&str]) -> String {
        let kernel_ver = crate::kernel::KERNEL_VERSION;
        let version = crate::kernel::VERSION;

        if args.contains(&"-a") {
            format!(
                "Linux kpawnd {} #1 SMP PREEMPT_DYNAMIC {} wasm32 GNU/Linux",
                kernel_ver, version
            )
        } else if args.contains(&"-r") {
            kernel_ver.into()
        } else if args.contains(&"-s") {
            "Linux".into()
        } else if args.contains(&"-n") {
            "kpawnd".into()
        } else if args.contains(&"-m") {
            "wasm32".into()
        } else if args.contains(&"-o") {
            "GNU/Linux".into()
        } else {
            "Linux".into()
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

    fn cmd_man(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "What manual page do you want?\nFor example, try 'man ls'.".into();
        }

        let cmd = args[0];
        match cmd {
            "ls" => {
                r#"LS(1)                            User Commands                           LS(1)

NAME
       ls - list directory contents

SYNOPSIS
       ls [OPTION]... [FILE]...

DESCRIPTION
       List information about the FILEs (the current directory by default).

       -a, --all
              do not ignore entries starting with .

       -l     use a long listing format

EXAMPLES
       ls -la /bin
              List all files in /bin with details

SEE ALSO
       dir(1), find(1)
"#
                .into()
            }

            "cat" => {
                r#"CAT(1)                           User Commands                          CAT(1)

NAME
       cat - concatenate files and print on the standard output

SYNOPSIS
       cat [FILE]...

DESCRIPTION
       Concatenate FILE(s) to standard output.

EXAMPLES
       cat /etc/passwd
              Display the contents of /etc/passwd
"#
                .into()
            }

            "cd" => {
                r#"CD(1)                            User Commands                           CD(1)

NAME
       cd - change the working directory

SYNOPSIS
       cd [DIR]

DESCRIPTION
       Change the current directory to DIR. The default DIR is the value of the
       HOME shell variable (usually /home/user).

       ..     Move to parent directory
       /      Move to root directory
"#
                .into()
            }

            "pwd" => {
                r#"PWD(1)                           User Commands                          PWD(1)

NAME
       pwd - print name of current/working directory

SYNOPSIS
       pwd

DESCRIPTION
       Print the full filename of the current working directory.
"#
                .into()
            }

            "rm" => {
                r#"RM(1)                            User Commands                           RM(1)

NAME
       rm - remove files or directories

SYNOPSIS
       rm [OPTION]... [FILE]...

DESCRIPTION
       rm removes each specified file. By default, it does not remove directories.

       -f, --force
              ignore nonexistent files and arguments

       -r, -R, --recursive
              remove directories and their contents recursively

WARNING
       Removing critical system files (like /bin/sh) will cause a kernel panic!
"#
                .into()
            }

            "mkdir" => {
                r#"MKDIR(1)                         User Commands                        MKDIR(1)

NAME
       mkdir - make directories

SYNOPSIS
       mkdir [DIRECTORY]...

DESCRIPTION
       Create the DIRECTORY(ies), if they do not already exist.
"#
                .into()
            }

            "touch" => {
                r#"TOUCH(1)                         User Commands                        TOUCH(1)

NAME
       touch - change file timestamps

SYNOPSIS
       touch [FILE]...

DESCRIPTION
       Update the access and modification times of each FILE to the current time.
       A FILE argument that does not exist is created empty.
"#
                .into()
            }

            "nano" => {
                r#"NANO(1)                          User Commands                         NANO(1)

NAME
       nano - Nano's ANOther editor, inspired by Pico

SYNOPSIS
       nano [FILE]

DESCRIPTION
       nano is a small and friendly editor.

KEY BINDINGS
       ^G     Display help text
       ^O     Write the current file to disk
       ^X     Exit nano

       Use arrow keys to navigate. Type to insert text.
"#
                .into()
            }

            "ps" => {
                r#"PS(1)                            User Commands                           PS(1)

NAME
       ps - report a snapshot of the current processes

SYNOPSIS
       ps

DESCRIPTION
       ps displays information about a selection of the active processes.

OUTPUT
       PID    Process ID
       PPID   Parent process ID
       STAT   Process state (R=running, S=sleeping, T=stopped, Z=zombie)
       CMD    Command name
"#
                .into()
            }

            "kill" => {
                r#"KILL(1)                          User Commands                         KILL(1)

NAME
       kill - send a signal to a process

SYNOPSIS
       kill PID

DESCRIPTION
       Send SIGTERM to the process with the given PID.
"#
                .into()
            }

            "uname" => {
                r#"UNAME(1)                         User Commands                        UNAME(1)

NAME
       uname - print system information

SYNOPSIS
       uname [OPTION]...

DESCRIPTION
       Print certain system information.

       -a, --all
              print all information

       -s, --kernel-name
              print the kernel name

       -r, --kernel-release
              print the kernel release

       -m, --machine
              print the machine hardware name

       -o, --operating-system
              print the operating system
"#
                .into()
            }

            "ping" => {
                r#"PING(1)                          User Commands                         PING(1)

NAME
       ping - send ICMP ECHO_REQUEST to network hosts

SYNOPSIS
       ping HOST

DESCRIPTION
       ping uses the ICMP protocol's mandatory ECHO_REQUEST datagram to elicit
       an ICMP ECHO_RESPONSE from a host or gateway.

NOTE
       This implementation uses HTTP to simulate ping via a proxy service.
"#
                .into()
            }

            "curl" => {
                r#"CURL(1)                          User Commands                         CURL(1)

NAME
       curl - transfer a URL

SYNOPSIS
       curl [options] URL

OPTIONS
       -X METHOD
              Specify request method (GET, POST, etc.)

       -I, --head
              Show response headers only

       -v     Verbose mode

EXAMPLES
       curl https://api.github.com
       curl -I https://example.com
"#
                .into()
            }

            "grep" => {
                r#"GREP(1)                          User Commands                         GREP(1)

NAME
       grep - print lines matching a pattern

SYNOPSIS
       grep PATTERN FILE

DESCRIPTION
       grep searches for PATTERN in each FILE and prints each line that matches.

EXAMPLES
       grep "error" /var/log/syslog
              Search for lines containing "error" in syslog
"#
                .into()
            }

            "find" => {
                r#"FIND(1)                          User Commands                         FIND(1)

NAME
       find - search for files in a directory hierarchy

SYNOPSIS
       find [PATH]

DESCRIPTION
       find recursively lists all files and directories under PATH.
       If PATH is omitted, the current directory is used.

EXAMPLES
       find /etc
              List all files under /etc
       find .
              List all files in current directory recursively
"#
                .into()
            }

            "wc" => {
                r#"WC(1)                            User Commands                           WC(1)

NAME
       wc - print newline, word, and byte counts

SYNOPSIS
       wc FILE

DESCRIPTION
       Print newline, word, and byte counts for FILE.

OUTPUT
       Lines, words, bytes, and filename
"#
                .into()
            }

            "head" => {
                r#"HEAD(1)                          User Commands                         HEAD(1)

NAME
       head - output the first part of files

SYNOPSIS
       head [-n NUM] FILE

DESCRIPTION
       Print the first 10 lines of FILE to standard output.
       With -n NUM, print the first NUM lines instead.

EXAMPLES
       head -n 5 /etc/passwd
              Show first 5 lines of passwd
"#
                .into()
            }

            "tail" => {
                r#"TAIL(1)                          User Commands                         TAIL(1)

NAME
       tail - output the last part of files

SYNOPSIS
       tail [-n NUM] FILE

DESCRIPTION
       Print the last 10 lines of FILE to standard output.
       With -n NUM, print the last NUM lines instead.

EXAMPLES
       tail -n 20 /var/log/syslog
              Show last 20 lines of syslog
"#
                .into()
            }

            "diff" => {
                r#"DIFF(1)                          User Commands                         DIFF(1)

NAME
       diff - compare files line by line

SYNOPSIS
       diff FILE1 FILE2

DESCRIPTION
       Compare FILE1 and FILE2 line by line.
"#
                .into()
            }

            "sort" => {
                r#"SORT(1)                          User Commands                         SORT(1)

NAME
       sort - sort lines of text files

SYNOPSIS
       sort FILE

DESCRIPTION
       Write sorted concatenation of FILE to standard output.
"#
                .into()
            }

            "uniq" => {
                r#"UNIQ(1)                          User Commands                         UNIQ(1)

NAME
       uniq - report or omit repeated lines

SYNOPSIS
       uniq FILE

DESCRIPTION
       Filter adjacent matching lines from FILE.
"#
                .into()
            }

            "which" => {
                r#"WHICH(1)                         User Commands                        WHICH(1)

NAME
       which - locate a command

SYNOPSIS
       which COMMAND

DESCRIPTION
       which returns the pathnames of the files that would be executed in the
       current environment if COMMAND was run.
"#
                .into()
            }

            "whereis" => {
                r#"WHEREIS(1)                       User Commands                      WHEREIS(1)

NAME
       whereis - locate the binary, source, and manual page files for a command

SYNOPSIS
       whereis COMMAND

DESCRIPTION
       whereis locates the binary, source and manual files for the specified
       command names.
"#
                .into()
            }

            "file" => {
                r#"FILE(1)                          User Commands                         FILE(1)

NAME
       file - determine file type

SYNOPSIS
       file FILE

DESCRIPTION
       file tests each argument in an attempt to classify it by examining
       file type, permissions, and contents.
"#
                .into()
            }

            "cp" => {
                r#"CP(1)                            User Commands                           CP(1)

NAME
       cp - copy files and directories

SYNOPSIS
       cp SOURCE DEST

DESCRIPTION
       Copy SOURCE to DEST.

NOTE
       Directory copying (-r) not yet implemented.
"#
                .into()
            }

            "mv" => {
                r#"MV(1)                            User Commands                           MV(1)

NAME
       mv - move (rename) files

SYNOPSIS
       mv SOURCE DEST

DESCRIPTION
       Rename SOURCE to DEST, or move SOURCE to DEST.
"#
                .into()
            }

            "chmod" => {
                r#"CHMOD(1)                         User Commands                        CHMOD(1)

NAME
       chmod - change file mode bits

SYNOPSIS
       chmod MODE FILE

DESCRIPTION
       chmod changes the file mode bits of FILE.

NOTE
       Permissions are simulated in kpawnd and don't affect file access.
"#
                .into()
            }

            "chown" => {
                r#"CHOWN(1)                         User Commands                        CHOWN(1)

NAME
       chown - change file owner and group

SYNOPSIS
       chown OWNER FILE

DESCRIPTION
       chown changes the user and/or group ownership of FILE.

NOTE
       Ownership changes are simulated in kpawnd.
"#
                .into()
            }

            "df" => {
                r#"DF(1)                            User Commands                           DF(1)

NAME
       df - report file system disk space usage

SYNOPSIS
       df

DESCRIPTION
       df displays the amount of disk space available on the file system.
"#
                .into()
            }

            "du" => {
                r#"DU(1)                            User Commands                           DU(1)

NAME
       du - estimate file space usage

SYNOPSIS
       du [PATH]

DESCRIPTION
       Summarize disk usage of PATH (or current directory).
"#
                .into()
            }

            "apt" | "apt-get" => {
                r#"APT(8)                      Package Management                         APT(8)

NAME
       apt - command-line interface for package management

SYNOPSIS
       apt [install|remove|update|upgrade|search] [PACKAGE]

DESCRIPTION
       apt provides a high-level interface for package management.

COMMANDS
       update     Update package list
       upgrade    Upgrade all packages
       install    Install package
       remove     Remove package
       search     Search for packages

NOTE
       This is a simulated package manager in kpawnd.
"#
                .into()
            }

            "top" => {
                r#"TOP(1)                           User Commands                          TOP(1)

NAME
       top - display Linux processes

SYNOPSIS
       top

DESCRIPTION
       The top program provides a dynamic real-time view of a running system.
       It displays system summary information and a list of processes.

NOTE
       Press q or Ctrl+C to exit (simulated in kpawnd).
"#
                .into()
            }

            "sudo" => {
                r#"SUDO(8)                     System Administration                     SUDO(8)

NAME
       sudo - execute a command as another user

SYNOPSIS
       sudo COMMAND

DESCRIPTION
       sudo allows permitted users to run commands as the superuser or another user.
       Password authentication is required. The session is cached for 5 minutes.

EXAMPLES
       sudo ls /root
              List files in root's home directory
       sudo rm -rf /boot/grub
              DANGER: Delete GRUB bootloader (will break boot!)
"#
                .into()
            }

            "echo" => {
                r#"ECHO(1)                          User Commands                         ECHO(1)

NAME
       echo - display a line of text

SYNOPSIS
       echo [STRING]...

DESCRIPTION
       Echo the STRING(s) to standard output.

SPECIAL
       echo github
              Opens the kpawnd GitHub page in a new tab
"#
                .into()
            }

            "clear" => {
                r#"CLEAR(1)                         User Commands                        CLEAR(1)

NAME
       clear - clear the terminal screen

SYNOPSIS
       clear

DESCRIPTION
       clear clears your screen if this is possible.
"#
                .into()
            }

            "history" => {
                r#"HISTORY(1)                       User Commands                      HISTORY(1)

NAME
       history - display command history

SYNOPSIS
       history

DESCRIPTION
       Display the history list with line numbers. Use arrow keys to navigate
       through previous commands.
"#
                .into()
            }

            "neofetch" => {
                r#"NEOFETCH(1)                      User Commands                     NEOFETCH(1)

NAME
       neofetch - display system information

SYNOPSIS
       neofetch

DESCRIPTION
       Neofetch is a command-line system information tool. It displays
       information about your operating system, software and hardware.
"#
                .into()
            }

            "python" => {
                r#"PYTHON(1)                        User Commands                       PYTHON(1)

NAME
       python - interactive Python interpreter

SYNOPSIS
       python

DESCRIPTION
       Start an interactive Python REPL (Read-Eval-Print Loop).
       This is a sandboxed Rust-backed Python interpreter.

       Type exit() to exit the interpreter.
"#
                .into()
            }

            "doom" => {
                r#"DOOM(1)                          User Commands                         DOOM(1)

        NAME
            doom - play a game

        SYNOPSIS
            doom [easy|normal|hard]

        DESCRIPTION
            Launch a simple game rendered onto a canvas.
            Optional difficulty adjusts monster count, damage, player HP.
            Press ESC to exit.

        DIFFICULTY
            easy    Fewer monsters, lower damage, higher player health
            normal  Balanced baseline (default)
            hard    More monsters, higher damage, lower player health
        "#
                .into()
            }

            "doommap" => {
                r#"DOOMMAP(1)                       User Commands                      DOOMMAP(1)

        NAME
            doommap - control procedural map generation for doom

        SYNOPSIS
            doommap proc
            doommap restore

        DESCRIPTION
            Enables or restores the original static map layout used by the Doom game.
            'proc' will generate a new procedural layout (rooms/corridors) without
            permanently destroying the original; 'restore' returns to the original map.

        "#
                .into()
            }

            "man" => {
                r#"MAN(1)                           User Commands                          MAN(1)

NAME
       man - an interface to the system reference manuals

SYNOPSIS
       man [COMMAND]

DESCRIPTION
       man is the system's manual pager. Each page argument given to man is
       normally the name of a program, utility or function.
"#
                .into()
            }

            "grub" => {
                r#"GRUB(1)                          User Commands                         GRUB(1)

NAME
       grub - manage bootloaders and simulate boot sequences

SYNOPSIS
       grub <switch|status|boot>

DESCRIPTION
       Manage the system's bootloader configuration and simulate boot processes.

       switch <bootloader>
              Switch to the specified bootloader (grub, systemd-boot)

       status
              Display current bootloader and list available bootloaders

       boot
              Simulate the boot sequence with visual animation

EXAMPLES
       grub status
              Show current bootloader configuration

       grub switch systemd-boot
              Switch to systemd-boot bootloader

       grub boot
              Start boot sequence simulation

SEE ALSO
       hasgrub(1)
"#
                .into()
            }

            _ => format!(
                "No manual entry for {}\n\nTry 'help' to see available commands.",
                cmd
            ),
        }
    }

    fn cmd_nano(&mut self, args: &[&str]) -> String {
        let filename = if args.is_empty() { "" } else { args[0] };
        let content = if !filename.is_empty() {
            match self.kernel.fs.resolve(filename) {
                Some(node) if !node.is_dir => node.data.clone(),
                Some(_) => return format!("nano: {}: Is a directory", filename),
                None => String::new(), // New file
            }
        } else {
            String::new()
        };
        format!("\x1b[NANO:{}:{}]", filename, content.replace('\n', "\\n"))
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
        let host = args.iter().rfind(|a| !a.starts_with('-')).unwrap_or(&"");

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

        if out.is_empty() {
            match filter {
                Some(name) => format!(
                    "{}: error fetching interface information: Device not found",
                    name
                ),
                None => out,
            }
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
                match self.kernel.proc.spawn(name, 1, &mut self.kernel.mem) {
                    Some(pid) => match self.services.start(name, pid) {
                        Ok(()) => format!("Started service '{}'", name),
                        Err(e) => format!("Error: {}", e),
                    },
                    None => "Failed to start service: out of memory".to_string(),
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
                match self.kernel.proc.spawn(name, 1, &mut self.kernel.mem) {
                    Some(pid) => match self.services.restart(name, pid) {
                        Ok(()) => format!("Restarted service '{}'", name),
                        Err(e) => format!("Error: {}", e),
                    },
                    None => "Failed to restart service: out of memory".to_string(),
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

    // Lightweight FS API for GUI explorer
    #[wasm_bindgen]
    pub fn fs_list(&self, path: &str) -> js_sys::Array {
        let arr = js_sys::Array::new();
        match self.kernel.fs.resolve(path) {
            Some(node) if node.is_dir => {
                let mut entries: Vec<_> = node.children.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                for (name, child) in entries {
                    let obj = js_sys::Object::new();
                    let _ = js_sys::Reflect::set(
                        &obj,
                        &JsValue::from_str("name"),
                        &JsValue::from_str(name),
                    );
                    let _ = js_sys::Reflect::set(
                        &obj,
                        &JsValue::from_str("is_dir"),
                        &JsValue::from_bool(child.is_dir),
                    );
                    let _ = js_sys::Reflect::set(
                        &obj,
                        &JsValue::from_str("size"),
                        &JsValue::from_f64(child.size as f64),
                    );
                    let _ = js_sys::Reflect::set(
                        &obj,
                        &JsValue::from_str("is_executable"),
                        &JsValue::from_bool(child.is_executable),
                    );
                    arr.push(&obj);
                }
            }
            _ => {}
        }
        arr
    }

    #[wasm_bindgen]
    pub fn fs_read(&self, path: &str) -> String {
        match self.kernel.fs.resolve(path) {
            Some(node) if !node.is_dir => node.data.clone(),
            _ => String::new(),
        }
    }

    #[wasm_bindgen]
    pub fn fs_write(&mut self, path: &str, data: &str) -> bool {
        self.kernel.fs.create_file(path, data).is_ok()
    }

    #[wasm_bindgen]
    pub fn fs_mkdir(&mut self, path: &str) -> bool {
        self.kernel.fs.create_dir(path).is_ok()
    }

    #[wasm_bindgen]
    pub fn fs_rm(&mut self, path: &str, recursive: bool) -> bool {
        if recursive {
            self.kernel.fs.remove_recursive(path).is_ok()
        } else {
            self.kernel.fs.remove(path).is_ok()
        }
    }
    #[wasm_bindgen]
    pub fn complete(&self, partial: &str) -> Vec<JsValue> {
        let mut matches = Vec::new();
        let cmds = [
            "apt", "awk", "cat", "cd", "chmod", "chown", "clear", "cp", "cut", "df", "diff", "du",
            "echo", "env", "exit", "export", "file", "find", "free", "grep", "gunzip", "gzip",
            "head", "help", "history", "hostname", "id", "kill", "ln", "ls", "mkdir", "mv", "ps",
            "pwd", "rm", "sed", "sort", "sudo", "tail", "tar", "tee", "top", "touch", "tr",
            "uname", "uniq", "unzip", "uptime", "wc", "whereis", "which", "whoami", "zip",
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

    #[wasm_bindgen]
    pub fn save_file(&mut self, path: &str, content: &str) -> String {
        // Check if file exists
        let normalized_path = self.kernel.fs.normalize(path);
        if self.kernel.fs.resolve(&normalized_path).is_some() {
            // Update existing file
            match self.kernel.fs.write_file(&normalized_path, content) {
                Ok(()) => String::new(),
                Err(e) => format!("Error saving file: {}", e),
            }
        } else {
            // Create new file
            match self.kernel.fs.create_file(path, content) {
                Ok(()) => String::new(),
                Err(e) => format!("Error creating file: {}", e),
            }
        }
    }

    /// Export user files as JSON for localStorage persistence
    #[wasm_bindgen]
    pub fn export_user_files(&self) -> String {
        self.kernel.fs.export_user_files()
    }

    /// Import user files from JSON (called on startup)
    #[wasm_bindgen]
    pub fn import_user_files(&mut self, json: &str) {
        self.kernel.fs.import_user_files(json);
    }

    #[wasm_bindgen]
    pub fn check_kernel_panic(&self) -> bool {
        self.kernel.fs.kernel_panic || self.kernel.memory_panic
    }

    #[wasm_bindgen]
    pub fn get_panic_message(&self) -> String {
        if self.kernel.memory_panic {
            self.kernel.memory_panic_reason.clone()
        } else {
            self.kernel.fs.panic_reason.clone()
        }
    }

    // Boot manager methods for JavaScript
    #[wasm_bindgen]
    pub fn boot_get_current_bootloader(&self) -> String {
        self.boot.get_current_bootloader().to_string()
    }

    #[wasm_bindgen]
    pub fn boot_list_bootloaders(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        for bootloader in self.boot.list_bootloaders() {
            arr.push(&JsValue::from_str(&bootloader));
        }
        arr
    }

    #[wasm_bindgen]
    pub fn boot_switch_bootloader(&mut self, name: &str) -> Result<(), JsValue> {
        self.boot.set_bootloader(name)
            .map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen]
    pub fn boot_simulate_sequence(&mut self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        for message in self.boot.simulate_boot_sequence(&mut self.kernel.mem) {
            arr.push(&JsValue::from_str(&message));
        }
        arr
    }

    /// Initialize system with persistence loading
    #[wasm_bindgen]
    pub async fn init(&mut self) {
        self.kernel.init().await;
    }

    /// Save system state to persistence
    #[wasm_bindgen]
    pub async fn save(&self) {
        self.kernel.save().await;
    }
}
