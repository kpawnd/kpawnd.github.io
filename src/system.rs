use crate::{
    boot::BootManager,
    kernel::Kernel,
    network::{NetworkStack, Protocol},
    process::{Priority, ProcState},
    python::PythonInterpreter,
    services::ServiceManager,
    shell::{prompt, Shell},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use wasm_bindgen::prelude::*;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const SUDO_TIMEOUT_MS: f64 = 300000.0;
const BINARY_PREFIX: &str = "__BIN_B64__:";

struct SudoPendingRequest {
    command: Option<String>,
    target_user: String,
    validate_only: bool,
    list_privileges: bool,
}

struct SudoInvocation {
    command: Option<String>,
    target_user: String,
    prompt: String,
    non_interactive: bool,
    reset_timestamp: bool,
    clear_timestamp: bool,
    validate_only: bool,
    list_privileges: bool,
}

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
    sudo_pending_request: Option<SudoPendingRequest>,
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
            sudo_pending_request: None,
            sudo_waiting_password: false,
            sudo_authenticated_until: None,
        };

        // Auto-start system services
        system.services.auto_start_services(&mut |name| {
            system.kernel.proc.spawn(name, 1, &mut system.kernel.mem)
        });

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
            self.sudo_waiting_password = false;
            if let Some(request) = self.sudo_pending_request.take() {
                return self.exec_sudo_with_context(
                    request.command.as_deref(),
                    trimmed,
                    &request.target_user,
                    request.validate_only,
                    request.list_privileges,
                );
            }
            return "sudo: authentication state is invalid; try again".into();
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return String::new();
        }
        let cmd = parts[0];
        let args = &parts[1..];
        if cmd == "sudo" {
            return self.handle_sudo(args);
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
            "help" => self.cmd_help(),
            "man" => self.cmd_man(args),
            "nano" | "vi" | "vim" => self.cmd_nano(args),
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
            "screensaver" => "\x1b[LAUNCH_SCREENSAVER]".to_string(),
            "wget" => self.cmd_wget(args),
            "curl" => self.cmd_curl(args),
            "myip" => self.cmd_myip(),
            "ls" => self.cmd_ls(args),
            "cd" => self.cmd_cd(args),
            "pwd" => self.kernel.fs.cwd.clone(),
            "cat" => self.cmd_cat(args),
            "grep" => self.cmd_grep(args),
            "find" => self.cmd_find(args),
            "wc" => self.cmd_wc(args),
            "head" => self.cmd_head(args),
            "tail" => self.cmd_tail(args),
            "diff" => self.cmd_diff(args),
            "sort" => self.cmd_sort(args),
            "uniq" => self.cmd_uniq(args),
            "cut" => self.cmd_cut(args),
            "tr" => self.cmd_tr(args),
            "tee" => self.cmd_tee(args),
            "which" => self.cmd_which(args),
            "whereis" => self.cmd_whereis(args),
            "file" => self.cmd_file(args),
            "ln" => self.cmd_ln(args),
            "cp" => self.cmd_cp(args),
            "mv" => self.cmd_mv(args),
            "chmod" => self.cmd_chmod(args),
            "chown" => self.cmd_chown(args),
            "df" => self.cmd_df(args),
            "du" => self.cmd_du(args),
            "tar" => self.cmd_tar(args),
            "gzip" | "gunzip" => self.cmd_gzip(args, cmd),
            "zip" | "unzip" => self.cmd_zip(args, cmd),
            "apt" | "apt-get" => self.cmd_apt(args),
            "top" => self.cmd_top(),
            "htop" => self.cmd_htop(),
            "awk" => self.cmd_awk(args),
            "sed" => self.cmd_sed(args),
            "alias" => self.cmd_alias(args),
            "touch" => self.cmd_touch(args),
            "mkdir" => self.cmd_mkdir(args),
            "rmdir" => self.cmd_rmdir(args),
            "rm" => self.cmd_rm(args),
            "clear" => "\x1b[CLEAR]".into(),
            "exit" => "\x1b[EXIT]".into(),
            "ps" => self.cmd_ps(),
            "kill" => self.cmd_kill(args),
            "uname" => self.cmd_uname(args),
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
            "date" => self.cmd_date(),
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
            "hasgrub" => if self.has_grub() { "yes".into() } else { "no".into() },
            "grub" => {
                if args.is_empty() {
                    return "\x1b[LAUNCH_GRUB]".into();
                }
                match args[0] {
                    "switch" => {
                        if args.len() < 2 {
                            return "usage: grub switch <bootloader>".into();
                        }
                        match self.boot.set_bootloader(args[1]) {
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
                        self.booted = true; // Mark system as booted for grub boot
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

    fn current_user(&self) -> String {
        self.shell
            .env
            .get("USER")
            .cloned()
            .unwrap_or_else(|| "user".into())
    }

    fn default_home_for_user(user: &str) -> String {
        if user == "root" {
            "/root".into()
        } else {
            format!("/home/{}", user)
        }
    }

    fn sudo_usage() -> String {
        "usage: sudo [-h|-K|-k|-V] [-nS] [-u user] [-p prompt] [-l|-v] [--] command".into()
    }

    fn parse_sudo_invocation(&self, args: &[&str]) -> Result<SudoInvocation, String> {
        if args.is_empty() {
            return Err(Self::sudo_usage());
        }

        let mut non_interactive = false;
        let mut prompt = "[sudo] password for %u:".to_string();
        let mut reset_timestamp = false;
        let mut clear_timestamp = false;
        let mut validate_only = false;
        let mut list_privileges = false;
        let mut target_user = "root".to_string();

        let mut i = 0;
        while i < args.len() {
            let arg = args[i];
            if arg == "--" {
                i += 1;
                break;
            }
            if !arg.starts_with('-') || arg == "-" {
                break;
            }

            if let Some(long_opt) = arg.strip_prefix("--") {
                match long_opt {
                    "help" => {
                        return Err("sudo - execute a command as another user\n\nusage: sudo [-h|-K|-k|-V] [-nS] [-u user] [-p prompt] [-l|-v] [--] command\n\nOptions:\n  -h, --help   show help\n  -V           show version\n  -k           invalidate authentication timestamp\n  -K           remove authentication timestamp entirely\n  -l           list current user's sudo privileges\n  -n           non-interactive; fail if password is required\n  -S           read password from terminal input\n  -u USER      run command as USER\n  -p PROMPT    use custom password prompt\n  -v           validate and refresh cached credentials".into());
                    }
                    "version" => {
                        return Err("Sudo version 1.9.14p3".into());
                    }
                    _ => return Err(format!("sudo: unknown option --{}", long_opt)),
                }
            } else {
                let chars: Vec<char> = arg[1..].chars().collect();
                let mut cidx = 0;
                while cidx < chars.len() {
                    match chars[cidx] {
                        'h' => {
                            return Err("sudo - execute a command as another user\n\nusage: sudo [-h|-K|-k|-V] [-nS] [-u user] [-p prompt] [-l|-v] [--] command\n\nOptions:\n  -h, --help   show help\n  -V           show version\n  -k           invalidate authentication timestamp\n  -K           remove authentication timestamp entirely\n  -l           list current user's sudo privileges\n  -n           non-interactive; fail if password is required\n  -S           read password from terminal input\n  -u USER      run command as USER\n  -p PROMPT    use custom password prompt\n  -v           validate and refresh cached credentials".into());
                        }
                        'V' => {
                            return Err("Sudo version 1.9.14p3".into());
                        }
                        'n' => non_interactive = true,
                        'k' => reset_timestamp = true,
                        'K' => {
                            clear_timestamp = true;
                            reset_timestamp = true;
                        }
                        'v' => validate_only = true,
                        'l' => list_privileges = true,
                        'S' => {
                            // Password still comes from terminal input in this shell model.
                        }
                        'u' => {
                            let trailing: String = chars[cidx + 1..].iter().collect();
                            if !trailing.is_empty() {
                                target_user = trailing;
                                cidx = chars.len();
                                continue;
                            }
                            if i + 1 >= args.len() {
                                return Err("sudo: option requires an argument -- 'u'".into());
                            }
                            target_user = args[i + 1].to_string();
                            i += 1;
                            break;
                        }
                        'p' => {
                            let trailing: String = chars[cidx + 1..].iter().collect();
                            if !trailing.is_empty() {
                                prompt = trailing;
                                cidx = chars.len();
                                continue;
                            }
                            if i + 1 >= args.len() {
                                return Err("sudo: option requires an argument -- 'p'".into());
                            }
                            prompt = args[i + 1].to_string();
                            i += 1;
                            break;
                        }
                        unknown => return Err(format!("sudo: unknown option -{}", unknown)),
                    }
                    cidx += 1;
                }
            }
            i += 1;
        }

        let command = if i < args.len() {
            Some(args[i..].join(" "))
        } else {
            None
        };

        Ok(SudoInvocation {
            command,
            target_user,
            prompt,
            non_interactive,
            reset_timestamp,
            clear_timestamp,
            validate_only,
            list_privileges,
        })
    }

    fn handle_sudo(&mut self, args: &[&str]) -> String {
        let parsed = match self.parse_sudo_invocation(args) {
            Ok(p) => p,
            Err(msg) => return msg,
        };

        if parsed.clear_timestamp || parsed.reset_timestamp {
            self.sudo_authenticated_until = None;
            if parsed.clear_timestamp {
                self.sudo_pending_request = None;
                self.sudo_waiting_password = false;
            }
        }

        if !parsed.validate_only && !parsed.list_privileges && parsed.command.is_none() {
            return Self::sudo_usage();
        }

        let current_user = self.current_user();
        let now = js_sys::Date::now();
        let is_authenticated = current_user == "root"
            || self
                .sudo_authenticated_until
                .map(|until| now < until)
                .unwrap_or(false);

        if is_authenticated {
            if parsed.list_privileges {
                return self.sudo_list_privileges();
            }
            if parsed.validate_only {
                self.sudo_authenticated_until = Some(now + SUDO_TIMEOUT_MS);
                return String::new();
            }
            return self.exec_sudo_internal(parsed.command.as_deref().unwrap_or(""), &parsed.target_user);
        }

        if parsed.non_interactive {
            return "sudo: a password is required".into();
        }

        let mut rendered_prompt = parsed.prompt;
        rendered_prompt = rendered_prompt.replace("%u", &current_user);
        rendered_prompt = rendered_prompt.replace("%U", &parsed.target_user);

        self.sudo_pending_request = Some(SudoPendingRequest {
            command: parsed.command,
            target_user: parsed.target_user,
            validate_only: parsed.validate_only,
            list_privileges: parsed.list_privileges,
        });
        self.sudo_waiting_password = true;

        if rendered_prompt.is_empty() {
            format!("[sudo] password for {}:", current_user)
        } else {
            rendered_prompt
        }
    }

    fn sudo_list_privileges(&self) -> String {
        let user = self.current_user();
        format!(
            "Matching Defaults entries for {} on kpawnd:\n    env_reset, mail_badpass, secure_path=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\n\nUser {} may run the following commands on kpawnd:\n    (ALL : ALL) ALL",
            user, user
        )
    }

    fn exec_sudo_internal(&mut self, cmd: &str, target_user: &str) -> String {
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

        let target_home = Self::default_home_for_user(target_user);

        self.shell.env.insert("USER".into(), target_user.into());
        self.shell.env.insert("HOME".into(), target_home.clone());
        let _ = self.kernel.fs.create_dir(&target_home);
        self.kernel.fs.set_default_owner(target_user, target_user);

        let out = self.exec(cmd);

        // revert
        self.shell.env.insert("USER".into(), old_user);
        self.shell.env.insert("HOME".into(), old_home);
        self.kernel.fs.set_default_owner(&old_owner, &old_group);
        out
    }

    fn exec_sudo_with_context(
        &mut self,
        cmd: Option<&str>,
        pw: &str,
        target_user: &str,
        validate_only: bool,
        list_privileges: bool,
    ) -> String {
        match &self.user_password {
            Some(saved) if saved == pw => {
                let now = js_sys::Date::now();
                self.sudo_authenticated_until = Some(now + SUDO_TIMEOUT_MS);
                if list_privileges {
                    self.sudo_list_privileges()
                } else if validate_only {
                    String::new()
                } else {
                    self.exec_sudo_internal(cmd.unwrap_or(""), target_user)
                }
            }
            _ => "sudo: 1 incorrect password attempt".into(),
        }
    }

    #[wasm_bindgen]
    pub fn exec_sudo(&mut self, cmd: &str, pw: &str) -> String {
        self.exec_sudo_with_context(Some(cmd), pw, "root", false, false)
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

        let path = args[0];
        match self.kernel.fs.resolve(path) {
            Some(n) if n.is_dir => format!("cat: {}: Is a directory", path),
            Some(n) if n.permissions.starts_with('l') => {
                let target = n.data.trim();
                match self.kernel.fs.resolve(target) {
                    Some(t) if t.is_dir => format!("cat: {}: Is a directory", target),
                    Some(t) => t.data.clone(),
                    None => format!("cat: {}: No such file or directory", target),
                }
            }
            Some(n) => n.data.clone(),
            None => format!("cat: {}: No such file or directory", path),
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

    fn cmd_rmdir(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "rmdir: missing operand".into();
        }
        for dir in args {
            match self.kernel.fs.resolve(dir) {
                Some(node) if !node.is_dir => {
                    return format!("rmdir: failed to remove '{}': Not a directory", dir);
                }
                Some(_) => match self.kernel.fs.remove(dir) {
                    Ok(()) => {}
                    Err(e) => return format!("rmdir: failed to remove '{}': {}", dir, e),
                },
                None => return format!("rmdir: failed to remove '{}': No such file or directory", dir),
            }
        }
        String::new()
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

    fn parse_list_spec(spec: &str) -> Result<Vec<usize>, String> {
        let mut selected = BTreeSet::new();
        for part in spec.split(',') {
            let token = part.trim();
            if token.is_empty() {
                continue;
            }
            if let Some((start, end)) = token.split_once('-') {
                let start_num = start
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| format!("invalid range start '{}'", start.trim()))?;
                let end_num = end
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| format!("invalid range end '{}'", end.trim()))?;
                if start_num == 0 || end_num == 0 || start_num > end_num {
                    return Err(format!("invalid range '{}'", token));
                }
                for idx in start_num..=end_num {
                    selected.insert(idx);
                }
            } else {
                let idx = token
                    .parse::<usize>()
                    .map_err(|_| format!("invalid field '{}'", token))?;
                if idx == 0 {
                    return Err("positions start at 1".to_string());
                }
                selected.insert(idx);
            }
        }
        if selected.is_empty() {
            return Err("empty list".to_string());
        }
        Ok(selected.into_iter().collect())
    }

    fn expand_tr_set(spec: &str) -> Vec<char> {
        let chars: Vec<char> = spec.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if i + 2 < chars.len() && chars[i + 1] == '-' {
                let start = chars[i] as u32;
                let end = chars[i + 2] as u32;
                if start <= end {
                    for c in start..=end {
                        if let Some(ch) = char::from_u32(c) {
                            out.push(ch);
                        }
                    }
                } else {
                    for c in end..=start {
                        if let Some(ch) = char::from_u32(c) {
                            out.push(ch);
                        }
                    }
                }
                i += 3;
            } else {
                out.push(chars[i]);
                i += 1;
            }
        }
        out
    }

    fn cmd_cut(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: cut (-f LIST [-d DELIM] | -c LIST) FILE".into();
        }

        let mut delim = '\t';
        let mut mode_chars = false;
        let mut list_spec: Option<String> = None;
        let mut file: Option<&str> = None;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "-d" => {
                    if i + 1 >= args.len() {
                        return "cut: option requires an argument -- 'd'".into();
                    }
                    delim = args[i + 1].chars().next().unwrap_or('\t');
                    i += 2;
                }
                "-f" => {
                    if i + 1 >= args.len() {
                        return "cut: option requires an argument -- 'f'".into();
                    }
                    mode_chars = false;
                    list_spec = Some(args[i + 1].to_string());
                    i += 2;
                }
                "-c" => {
                    if i + 1 >= args.len() {
                        return "cut: option requires an argument -- 'c'".into();
                    }
                    mode_chars = true;
                    list_spec = Some(args[i + 1].to_string());
                    i += 2;
                }
                value if value.starts_with('-') => {
                    return format!("cut: invalid option -- '{}'", value);
                }
                value => {
                    file = Some(value);
                    i += 1;
                }
            }
        }

        let file = match file {
            Some(f) => f,
            None => return "cut: missing file operand".into(),
        };
        let list_spec = match list_spec {
            Some(s) => s,
            None => return "cut: one of -f or -c must be specified".into(),
        };

        let list = match Self::parse_list_spec(&list_spec) {
            Ok(v) => v,
            Err(e) => return format!("cut: invalid list value: {}", e),
        };

        let node = match self.kernel.fs.resolve(file) {
            Some(n) if n.is_dir => return format!("cut: {}: Is a directory", file),
            Some(n) => n,
            None => return format!("cut: {}: No such file or directory", file),
        };

        let mut out = Vec::new();
        for line in node.data.lines() {
            if mode_chars {
                let chars: Vec<char> = line.chars().collect();
                let selected: String = list
                    .iter()
                    .filter_map(|pos| chars.get(pos - 1).copied())
                    .collect();
                out.push(selected);
            } else {
                let fields: Vec<&str> = line.split(delim).collect();
                let mut selected = Vec::new();
                for pos in &list {
                    if let Some(field) = fields.get(pos - 1) {
                        selected.push(*field);
                    }
                }
                out.push(selected.join(&delim.to_string()));
            }
        }
        out.join("\n")
    }

    fn cmd_tr(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: tr [-d] SET1 [SET2] <text> | tr [-d] SET1 [SET2] -f FILE".into();
        }

        let mut delete_mode = false;
        let mut idx = 0;
        if args[idx] == "-d" {
            delete_mode = true;
            idx += 1;
        }

        if idx >= args.len() {
            return "tr: missing SET1 operand".into();
        }
        let set1 = Self::expand_tr_set(args[idx]);
        idx += 1;

        let set2 = if delete_mode {
            Vec::new()
        } else {
            if idx >= args.len() {
                return "tr: missing SET2 operand".into();
            }
            let s = Self::expand_tr_set(args[idx]);
            idx += 1;
            if s.is_empty() {
                return "tr: SET2 cannot be empty".into();
            }
            s
        };

        let input = if idx < args.len() && args[idx] == "-f" {
            if idx + 1 >= args.len() {
                return "tr: -f requires a file path".into();
            }
            let path = args[idx + 1];
            match self.kernel.fs.resolve(path) {
                Some(n) if n.is_dir => return format!("tr: {}: Is a directory", path),
                Some(n) => n.data.clone(),
                None => return format!("tr: {}: No such file or directory", path),
            }
        } else {
            if idx >= args.len() {
                return "tr: missing input text (or use -f FILE)".into();
            }
            args[idx..].join(" ")
        };

        let mut out = String::new();
        for ch in input.chars() {
            if let Some(pos) = set1.iter().position(|c| *c == ch) {
                if delete_mode {
                    continue;
                }
                let mapped = set2.get(pos).copied().unwrap_or(*set2.last().unwrap());
                out.push(mapped);
            } else {
                out.push(ch);
            }
        }
        out
    }

    fn cmd_tee(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: tee [-a] FILE [TEXT ...] | tee [-a] FILE -f INPUT_FILE".into();
        }

        let mut append = false;
        let mut idx = 0;
        if args[idx] == "-a" {
            append = true;
            idx += 1;
        }
        if idx >= args.len() {
            return "tee: missing FILE operand".into();
        }
        let out_path = args[idx];
        idx += 1;

        let text = if idx < args.len() && args[idx] == "-f" {
            if idx + 1 >= args.len() {
                return "tee: -f requires INPUT_FILE".into();
            }
            let input_path = args[idx + 1];
            match self.kernel.fs.resolve(input_path) {
                Some(n) if n.is_dir => return format!("tee: {}: Is a directory", input_path),
                Some(n) => n.data.clone(),
                None => return format!("tee: {}: No such file or directory", input_path),
            }
        } else {
            args[idx..].join(" ")
        };

        let final_data = if append {
            match self.kernel.fs.resolve(out_path) {
                Some(n) if n.is_dir => return format!("tee: {}: Is a directory", out_path),
                Some(n) => {
                    if n.data.is_empty() {
                        text.clone()
                    } else if text.is_empty() {
                        n.data.clone()
                    } else {
                        format!("{}\n{}", n.data, text)
                    }
                }
                None => text.clone(),
            }
        } else {
            text.clone()
        };

        let write_result = match self.kernel.fs.resolve(out_path) {
            Some(n) if n.is_dir => Err("is a directory"),
            Some(_) => self.kernel.fs.write_file(out_path, &final_data),
            None => self.kernel.fs.create_file(out_path, &final_data),
        };

        match write_result {
            Ok(()) => text,
            Err(e) => format!("tee: {}: {}", out_path, e),
        }
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

    fn cmd_ln(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: ln [-s] TARGET LINK_NAME".into();
        }

        let mut symbolic = false;
        let mut idx = 0;
        if args[idx] == "-s" {
            symbolic = true;
            idx += 1;
        }
        if args.len() - idx < 2 {
            return "ln: missing file operand\nusage: ln [-s] TARGET LINK_NAME".into();
        }

        let target = args[idx];
        let link_name = args[idx + 1];

        if self.kernel.fs.resolve(link_name).is_some() {
            return format!("ln: failed to create link '{}': File exists", link_name);
        }

        if symbolic {
            match self.kernel.fs.create_file(link_name, target) {
                Ok(()) => {
                    if let Some(node) = self.kernel.fs.resolve_mut(link_name) {
                        node.permissions = "lrwxrwxrwx".into();
                        node.size = target.len();
                        node.is_executable = false;
                    }
                    String::new()
                }
                Err(e) => format!("ln: failed to create symbolic link '{}': {}", link_name, e),
            }
        } else {
            let source = match self.kernel.fs.resolve(target) {
                Some(n) if n.is_dir => {
                    return format!("ln: hard link not allowed for directory '{}'", target);
                }
                Some(n) => n.clone(),
                None => return format!("ln: failed to access '{}': No such file or directory", target),
            };

            match self.kernel.fs.create_file(link_name, &source.data) {
                Ok(()) => {
                    if let Some(node) = self.kernel.fs.resolve_mut(link_name) {
                        node.permissions = source.permissions;
                        node.owner = source.owner;
                        node.group = source.group;
                        node.is_executable = source.is_executable;
                        node.size = source.size;
                    }
                    String::new()
                }
                Err(e) => format!("ln: failed to create link '{}': {}", link_name, e),
            }
        }
    }

    fn cmd_date(&self) -> String {
        let d = js_sys::Date::new_0();
        d.to_string().into()
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
        "chmod: mode changes are not yet wired to permission enforcement".into()
    }

    fn cmd_chown(&mut self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "usage: chown [owner] [file]".into();
        }
        "chown: owner/group metadata updates are not yet wired".into()
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

    fn read_file_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        let node = self
            .kernel
            .fs
            .resolve(path)
            .ok_or_else(|| format!("{}: No such file or directory", path))?;
        if node.is_dir {
            return Err(format!("{}: Is a directory", path));
        }
        if let Some(encoded) = node.data.strip_prefix(BINARY_PREFIX) {
            return B64
                .decode(encoded)
                .map_err(|_| format!("{}: Invalid encoded binary data", path));
        }
        Ok(node.data.as_bytes().to_vec())
    }

    fn write_file_bytes(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        let content = match String::from_utf8(bytes.to_vec()) {
            Ok(text) => text,
            Err(_) => format!("{}{}", BINARY_PREFIX, B64.encode(bytes)),
        };

        if self.kernel.fs.resolve(path).is_some() {
            self.kernel
                .fs
                .write_file(path, &content)
                .map_err(|e| e.to_string())
        } else {
            self.kernel
                .fs
                .create_file(path, &content)
                .map_err(|e| e.to_string())
        }
    }

    fn ensure_dir_all(&mut self, path: &str) -> Result<(), String> {
        let normalized = self.kernel.fs.normalize(path);
        if normalized == "/" {
            return Ok(());
        }

        let mut current = String::new();
        for part in normalized.split('/').filter(|p| !p.is_empty()) {
            current.push('/');
            current.push_str(part);
            if self.kernel.fs.resolve(&current).is_none() {
                self.kernel
                    .fs
                    .create_dir(&current)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn join_virtual_path(base: &str, name: &str) -> String {
        if base == "/" {
            format!("/{}", name.trim_start_matches('/'))
        } else {
            format!(
                "{}/{}",
                base.trim_end_matches('/'),
                name.trim_start_matches('/')
            )
        }
    }

    fn cmd_tar(&self, _args: &[&str]) -> String {
        "tar: use zip/unzip for archive workflows in this build".into()
    }

    fn cmd_gzip(&mut self, args: &[&str], cmd: &str) -> String {
        let mut keep_input = false;
        let mut files: Vec<&str> = Vec::new();

        for arg in args {
            match *arg {
                "-k" | "--keep" => keep_input = true,
                other if other.starts_with('-') => {
                    return format!("{}: unsupported option '{}'", cmd, other);
                }
                other => files.push(other),
            }
        }

        if files.is_empty() {
            return if cmd == "gzip" {
                "usage: gzip [-k] FILE...".into()
            } else {
                "usage: gunzip [-k] FILE...".into()
            };
        }

        let mut out_lines = Vec::new();

        for path in files {
            if cmd == "gzip" {
                let input = match self.read_file_bytes(path) {
                    Ok(data) => data,
                    Err(e) => return format!("gzip: {}", e),
                };
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                if encoder.write_all(&input).is_err() {
                    return format!("gzip: {}: write failure", path);
                }
                let compressed = match encoder.finish() {
                    Ok(data) => data,
                    Err(_) => return format!("gzip: {}: compression failure", path),
                };
                let out_path = format!("{}.gz", path);
                if let Err(e) = self.write_file_bytes(&out_path, &compressed) {
                    return format!("gzip: {}: {}", out_path, e);
                }
                if !keep_input {
                    let _ = self.kernel.fs.remove(path);
                }
                out_lines.push(format!("{} -> {}", path, out_path));
            } else {
                let input = match self.read_file_bytes(path) {
                    Ok(data) => data,
                    Err(e) => return format!("gunzip: {}", e),
                };
                let mut decoder = GzDecoder::new(Cursor::new(input));
                let mut decompressed = Vec::new();
                if decoder.read_to_end(&mut decompressed).is_err() {
                    return format!("gunzip: {}: invalid gzip stream", path);
                }
                let out_path = if path.ends_with(".gz") {
                    path.trim_end_matches(".gz").to_string()
                } else {
                    format!("{}.out", path)
                };
                if let Err(e) = self.write_file_bytes(&out_path, &decompressed) {
                    return format!("gunzip: {}: {}", out_path, e);
                }
                if !keep_input {
                    let _ = self.kernel.fs.remove(path);
                }
                out_lines.push(format!("{} -> {}", path, out_path));
            }
        }

        out_lines.join("\n")
    }

    fn cmd_zip(&mut self, args: &[&str], cmd: &str) -> String {
        if cmd == "zip" {
            let mut recursive = false;
            let mut archive_path = None;
            let mut sources: Vec<&str> = Vec::new();

            for arg in args {
                match *arg {
                    "-r" => recursive = true,
                    other if other.starts_with('-') => return format!("zip: unsupported option '{}'", other),
                    other => {
                        if archive_path.is_none() {
                            archive_path = Some(other);
                        } else {
                            sources.push(other);
                        }
                    }
                }
            }

            let archive_path = match archive_path {
                Some(v) => v,
                None => return "usage: zip [-r] ARCHIVE.zip FILE...".into(),
            };
            if sources.is_empty() {
                return "zip: nothing to do".into();
            }

            let mut sink = Cursor::new(Vec::<u8>::new());
            {
                let mut writer = ZipWriter::new(&mut sink);
                let opts = SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .unix_permissions(0o644);

                let mut queue: Vec<String> = sources.iter().map(|s| self.kernel.fs.normalize(s)).collect();
                while let Some(path) = queue.pop() {
                    let node = match self.kernel.fs.resolve(&path) {
                        Some(n) => n,
                        None => return format!("zip: name not matched: {}", path),
                    };
                    if node.is_dir {
                        if !recursive {
                            return format!("zip: {} is a directory (try -r)", path);
                        }
                        let entry_name = path.trim_start_matches('/').trim_end_matches('/').to_string() + "/";
                        if !entry_name.is_empty() {
                            if writer.add_directory(entry_name, opts).is_err() {
                                return format!("zip: failed to add directory {}", path);
                            }
                        }
                        for child in node.children.keys() {
                            queue.push(Self::join_virtual_path(&path, child));
                        }
                    } else {
                        let entry_name = path.trim_start_matches('/');
                        if writer.start_file(entry_name, opts).is_err() {
                            return format!("zip: failed to add {}", path);
                        }
                        let data = match self.read_file_bytes(&path) {
                            Ok(d) => d,
                            Err(e) => return format!("zip: {}", e),
                        };
                        if writer.write_all(&data).is_err() {
                            return format!("zip: failed writing {}", path);
                        }
                    }
                }

                if writer.finish().is_err() {
                    return "zip: finalize failed".into();
                }
            }

            let bytes = sink.into_inner();
            match self.write_file_bytes(archive_path, &bytes) {
                Ok(()) => format!("created {}", archive_path),
                Err(e) => format!("zip: {}", e),
            }
        } else {
            if args.is_empty() {
                return "usage: unzip ARCHIVE.zip [-d DIR]".into();
            }

            let mut archive_path = None;
            let mut out_dir = ".".to_string();
            let mut i = 0;
            while i < args.len() {
                match args[i] {
                    "-d" => {
                        if i + 1 >= args.len() {
                            return "unzip: option requires an argument -- 'd'".into();
                        }
                        out_dir = args[i + 1].to_string();
                        i += 2;
                    }
                    value if value.starts_with('-') => {
                        return format!("unzip: unsupported option '{}'", value)
                    }
                    value => {
                        archive_path = Some(value.to_string());
                        i += 1;
                    }
                }
            }

            let archive_path = match archive_path {
                Some(v) => v,
                None => return "usage: unzip ARCHIVE.zip [-d DIR]".into(),
            };

            let archive_bytes = match self.read_file_bytes(&archive_path) {
                Ok(d) => d,
                Err(e) => return format!("unzip: {}", e),
            };

            let cursor = Cursor::new(archive_bytes);
            let mut archive = match ZipArchive::new(cursor) {
                Ok(a) => a,
                Err(_) => return format!("unzip: {}: invalid zip archive", archive_path),
            };

            let mut created = Vec::new();
            for idx in 0..archive.len() {
                let mut entry = match archive.by_index(idx) {
                    Ok(e) => e,
                    Err(_) => return "unzip: failed reading archive entry".into(),
                };
                let name = entry.name().to_string();
                if name.starts_with('/') || name.contains("..") {
                    continue;
                }
                let dest = Self::join_virtual_path(&self.kernel.fs.normalize(&out_dir), &name);
                if entry.is_dir() {
                    if let Err(e) = self.ensure_dir_all(&dest) {
                        return format!("unzip: {}", e);
                    }
                    continue;
                }

                if let Some((parent, _)) = dest.rsplit_once('/') {
                    let parent_dir = if parent.is_empty() { "/" } else { parent };
                    if let Err(e) = self.ensure_dir_all(parent_dir) {
                        return format!("unzip: {}", e);
                    }
                }

                let mut data = Vec::new();
                if entry.read_to_end(&mut data).is_err() {
                    return format!("unzip: failed extracting {}", name);
                }
                if let Err(e) = self.write_file_bytes(&dest, &data) {
                    return format!("unzip: {}", e);
                }
                created.push(dest);
            }

            if created.is_empty() {
                "Archive processed: no entries extracted".into()
            } else {
                created.join("\n")
            }
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

    fn cmd_htop(&self) -> String {
        let total_mem = self.kernel.mem.total;
        let free_mem = self.kernel.mem.free;
        let used_mem = total_mem - free_mem;
        let mem_pct = if total_mem > 0 {
            (used_mem as f64 / total_mem as f64) * 100.0
        } else {
            0.0
        };

        let proc_list = self.kernel.proc.list();
        let task_total = proc_list.len();
        let task_sleep = task_total.saturating_sub(1);
        let uptime_s = self.kernel.uptime_ms() / 1000;

        let mut out = String::new();
        out.push_str(&format!(
            "htop - kpawnd Linux | uptime {}s\nTasks: {} total, 1 running, {} sleeping\nCPU: [#---------] 3.0%  MEM: [{:<10}] {:>5.1}%  SWP: [----------] 0.0%\n\n",
            uptime_s,
            task_total,
            task_sleep,
            "#".repeat(((mem_pct / 10.0).round() as usize).min(10)),
            mem_pct
        ));
        out.push_str(" PID  USER      PRI  NI   VIRT   RES   SHR S  CPU% MEM%  TIME+  COMMAND\n");

        for p in proc_list {
            let state = match p.state {
                ProcState::Run => "R",
                ProcState::Sleep => "S",
                ProcState::Stop => "T",
                ProcState::Zombie => "Z",
            };
            out.push_str(&format!(
                "{:>4} {:<8} {:>4} {:>3} {:>6} {:>5} {:>5} {} {:>5.1} {:>4.1} {:>6}  {}\n",
                p.pid,
                "user",
                20,
                0,
                "32M",
                "8M",
                "2M",
                state,
                if state == "R" { 3.0 } else { 0.1 },
                if total_mem > 0 { (8.0 * 1024.0 * 1024.0 / total_mem as f64) * 100.0 } else { 0.0 },
                "00:00",
                p.name
            ));
        }

        out.push_str("\nHint: use `top` for compact view, `ps` for snapshot.");
        out
    }

    fn cmd_help(&self) -> String {
        "kpawnd terminal help\n\nCore filesystem:\n  ls cd pwd cat cp mv rm rmdir mkdir touch ln file find\n\nText processing:\n  grep awk sed sort uniq wc head tail cut tr tee diff\n\nSystem and process:\n  ps top htop kill free df du uname hostname id whoami\n  uptime date env export history clear\n\nNetwork:\n  ip ifconfig route arp ss netstat ping traceroute host dig\n  nslookup curl wget nc myip\n\nTooling and shell:\n  man which whereis alias sudo python nano vi service\n\nBoot and extras:\n  grub hasgrub reboot screensaver doom doommap\n\nQoL:\n  Tab autocomplete, ArrowUp/ArrowDown history, Ctrl+L clear line, Ctrl+C cancel line\n  man -k <term> to search docs\n\nUse `man <command>` for details.".into()
    }

    fn cmd_awk(&self, _args: &[&str]) -> String {
        "awk: currently supports only the parser shell path; program execution is pending".into()
    }

    fn cmd_sed(&self, _args: &[&str]) -> String {
        "sed: currently supports only the parser shell path; script execution is pending".into()
    }

    fn cmd_alias(&self, args: &[&str]) -> String {
        if args.is_empty() {
            "alias ls='ls --color=auto'\nalias ll='ls -la'\nalias la='ls -A'\nalias l='ls -CF'"
                .into()
        } else {
            "alias: runtime alias mutation is pending".into()
        }
    }

    fn is_builtin(&self, cmd: &str) -> bool {
        matches!(
            cmd,
            "alias"
                | "apt"
                | "apt-get"
                | "arp"
                | "awk"
                | "cat"
                | "cd"
                | "chmod"
                | "chown"
                | "clear"
                | "cp"
                | "curl"
                | "cut"
                | "date"
                | "df"
                | "diff"
                | "dig"
                | "doom"
                | "doommap"
                | "du"
                | "echo"
                | "env"
                | "exit"
                | "export"
                | "file"
                | "find"
                | "free"
                | "grub"
                | "grep"
                | "gunzip"
                | "gzip"
                | "hasgrub"
                | "head"
                | "help"
                | "history"
                | "host"
                | "hostname"
                | "htop"
                | "id"
                | "ifconfig"
                | "ip"
                | "kill"
                | "ln"
                | "ls"
                | "man"
                | "mkdir"
                | "mv"
                | "myip"
                | "nano"
                | "nc"
                | "netcat"
                | "netstat"
                | "nslookup"
                | "ping"
                | "ps"
                | "pwd"
                | "python"
                | "reboot"
                | "rm"
                | "rmdir"
                | "route"
                | "screensaver"
                | "sed"
                | "service"
                | "socket"
                | "sort"
                | "ss"
                | "sudo"
                | "tail"
                | "tar"
                | "tee"
                | "top"
                | "touch"
                | "tr"
                | "traceroute"
                | "tracert"
                | "uname"
                | "uniq"
                | "unzip"
                | "uptime"
                | "vi"
                | "vim"
                | "wc"
                | "wget"
                | "whereis"
                | "which"
                | "whoami"
                | "zip"
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
            return "man - Linux manual pager (kpawnd)\n\nUsage:\n  man <command>\n  man -k <keyword>\n\nExamples:\n  man ls\n  man htop\n  man -k network\n\nTip: run `help` to list all available commands.".into();
        }

        if args[0] == "-k" {
            if args.len() < 2 {
                return "man: what keyword?\nusage: man -k <keyword>".into();
            }
            let needle = args[1].to_lowercase();
            let pages = [
                "alias", "apt", "cat", "cd", "chmod", "chown", "clear", "cp", "curl", "cut", "date",
                "df", "diff", "du", "echo", "find", "free", "grep", "head", "help", "history", "host",
                "hostname", "htop", "id", "kill", "ln", "ls", "man", "mkdir", "mv", "ping", "ps",
                "pwd", "python", "rm", "sort", "sudo", "tail", "tee", "top", "touch", "tr", "uname",
                "uniq", "uptime", "wc", "whereis", "which", "whoami", "grub", "doom", "doommap",
            ];
            let matches: Vec<&str> = pages
                .iter()
                .copied()
                .filter(|name| name.contains(&needle))
                .collect();
            if matches.is_empty() {
                return format!("man: nothing appropriate for '{}'", args[1]);
            }
            return matches
                .iter()
                .map(|name| format!("{} (1) - manual entry", name))
                .collect::<Vec<_>>()
                .join("\n");
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
    Permission metadata is shown, but access-control checks are permissive.
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
    Ownership metadata updates are accepted by the virtual filesystem.
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
    Package output is sourced from the built-in package database.
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
    Press q or Ctrl+C to exit.
"#
                .into()
            }

            "sudo" => {
                r#"SUDO(8)                     System Administration                     SUDO(8)

NAME
       sudo - execute a command as another user

SYNOPSIS
    sudo [-h|-K|-k|-V] [-nS] [-u user] [-p prompt] [-l|-v] [--] command

DESCRIPTION
       sudo allows permitted users to run commands as the superuser or another user.
       Password authentication is required. The session is cached for 5 minutes.

OPTIONS
    -u USER
        run command as USER (default: root)

    -n
        non-interactive mode; fail instead of prompting for a password

    -k, -K
        invalidate cached credentials (timestamp)

    -v
        validate credentials and refresh sudo timestamp

    -l
        list effective sudo privileges

    -p PROMPT
        set custom password prompt (supports %u and %U)

EXAMPLES
       sudo ls /root
              List files in root's home directory
    sudo -u user ls /home/user
        Run command as a non-root target user
    sudo -l
        Show sudo privileges for current user
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

                 "htop" => {
                  r#"HTOP(1)                          User Commands                         HTOP(1)

NAME
            htop - interactive process viewer

SYNOPSIS
            htop

DESCRIPTION
            htop shows a richer process and resource view than top:
            CPU summary, memory bar, and process table.

            In kpawnd this is a non-interactive snapshot designed to match htop style.
"#
                .into()
            }

                 "help" => {
                  r#"HELP(1)                          User Commands                         HELP(1)

        NAME
            help - show command groups and shell quality-of-life features

        SYNOPSIS
            help

        DESCRIPTION
            Displays grouped commands (filesystem, text, process, network, tooling)
            and built-in shortcuts such as Tab completion and history navigation.

        SEE ALSO
            man(1), which(1), whereis(1)
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

            With no arguments, grub opens the GRUB boot menu. The menu supports
            arrow-key selection, Enter to boot, e to edit the selected entry, and
            c for the GRUB command line.

       switch <bootloader>
              Switch to the specified bootloader (grub, systemd-boot)

       status
              Display current bootloader and list available bootloaders

       boot
              Simulate the boot sequence with visual animation

    The boot menu mirrors a classic GRUB layout with a timeout, submenu
    navigation, edit mode, and a command-line prompt.

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

        // Include baseline listening sockets.
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
            "alias", "apt", "apt-get", "arp", "awk", "cat", "cd", "chmod", "chown", "clear", "cp",
            "curl", "cut", "date", "df", "diff", "dig", "doom", "doommap", "du", "echo", "env",
            "exit", "export", "file", "find", "free", "grep", "grub", "gunzip", "gzip", "hasgrub",
            "head", "help", "history", "host", "hostname", "htop", "id", "ifconfig", "ip", "kill",
            "ln", "ls", "man", "mkdir", "mv", "myip", "nano", "nc", "netcat", "netstat", "nslookup",
            "ping", "ps", "pwd", "python", "reboot", "rm", "rmdir", "route", "screensaver", "sed",
            "service", "socket", "sort", "ss", "sudo", "tail", "tar", "tee", "top", "touch", "tr",
            "traceroute", "tracert", "uname", "uniq", "unzip", "uptime", "vi", "vim", "wc", "wget",
            "whereis", "which", "whoami", "zip",
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
    pub fn complete_path(&self, partial: &str) -> Vec<JsValue> {
        let mut out = Vec::new();
        let normalized = self.kernel.fs.normalize(partial);

        let (parent_path, needle) = match normalized.rfind('/') {
            Some(0) => ("/".to_string(), normalized[1..].to_string()),
            Some(idx) => (
                normalized[..idx].to_string(),
                normalized[idx + 1..].to_string(),
            ),
            None => (self.kernel.fs.cwd.clone(), normalized),
        };

        if let Some(parent) = self.kernel.fs.resolve(&parent_path) {
            if parent.is_dir {
                for (name, inode) in &parent.children {
                    if name.starts_with(&needle) {
                        let suggestion = if parent_path == "/" {
                            format!("/{}{}", name, if inode.is_dir { "/" } else { "" })
                        } else if partial.contains('/') {
                            format!(
                                "{}/{}{}",
                                parent_path,
                                name,
                                if inode.is_dir { "/" } else { "" }
                            )
                        } else {
                            format!("{}{}", name, if inode.is_dir { "/" } else { "" })
                        };
                        out.push(JsValue::from_str(&suggestion));
                    }
                }
            }
        }

        out
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
    pub fn memory_total_kb(&self) -> u32 {
        self.kernel.mem.total / 1024
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
        self.boot
            .set_bootloader(name)
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
