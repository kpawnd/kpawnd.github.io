use crate::{
    kernel::Kernel,
    process::ProcState,
    python::PythonInterpreter,
    shell::{prompt, Shell},
    vfs::Inode,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct System {
    kernel: Kernel,
    shell: Shell,
    booted: bool,
    cleared_after_boot: bool,
    python_interp: Option<PythonInterpreter>,
    in_python_repl: bool,
}

#[wasm_bindgen]
impl System {
    #[wasm_bindgen(constructor)]
    pub fn new() -> System {
        System {
            kernel: Kernel::new(),
            shell: Shell::new(),
            booted: false,
            cleared_after_boot: false,
            python_interp: None,
            in_python_repl: false,
        }
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
        self.kernel.scheduler.tick();
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
            self.kernel.scheduler.add(pid);
        }
        match cmd {
            "echo" => { let out=args.join(" "); if out=="github" { format!("\x1b[OPEN:{}]", self.shell.env.get("GITHUB").unwrap()) } else { out } }
            "help" => "cat cd clear echo exit help hostname id ls mkdir neofetch ps pwd rm touch uname uptime free env export kill history python".into(),
            "neofetch" => "\x1b[NEOFETCH_DATA]".to_string(),
            "python" => { if args.is_empty() { self.start_python_repl() } else { format!("python: script execution not supported") } }
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
            "" => String::new(),
            _ => format!("sh: {}: command not found", cmd),
        }
    }

    fn cmd_ls(&self, args: &[&str]) -> String {
        let path = args.get(0).unwrap_or(&".");
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
        let target = args.get(0).unwrap_or(&"/");
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
