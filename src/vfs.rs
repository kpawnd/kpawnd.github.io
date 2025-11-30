use std::collections::HashMap;

// Critical system binaries that will crash if deleted
pub const CRITICAL_BINARIES: &[&str] = &["sh", "bash", "init", "login", "getty"];
pub const IMPORTANT_BINARIES: &[&str] =
    &["ls", "cat", "cd", "pwd", "rm", "mkdir", "touch", "cp", "mv"];

#[derive(Clone)]
pub struct Inode {
    pub name: String,
    pub is_dir: bool,
    pub data: String,
    pub children: HashMap<String, Inode>,
    pub permissions: String,
    pub owner: String,
    pub group: String,
    pub size: usize,
    pub is_executable: bool,
    pub is_critical: bool,
}

impl Inode {
    pub fn dir(name: &str) -> Self {
        Inode {
            name: name.into(),
            is_dir: true,
            data: String::new(),
            children: HashMap::new(),
            permissions: "drwxr-xr-x".into(),
            owner: "root".into(),
            group: "root".into(),
            size: 4096,
            is_executable: false,
            is_critical: false,
        }
    }
    pub fn file(name: &str, data: &str) -> Self {
        Inode {
            name: name.into(),
            is_dir: false,
            data: data.into(),
            children: HashMap::new(),
            permissions: "-rw-r--r--".into(),
            owner: "root".into(),
            group: "root".into(),
            size: data.len(),
            is_executable: false,
            is_critical: false,
        }
    }
    pub fn binary(name: &str, desc: &str, critical: bool) -> Self {
        Inode {
            name: name.into(),
            is_dir: false,
            data: format!(
                "#!/bin/sh\n# {} - {}\n# ELF 64-bit LSB executable, x86-64",
                name, desc
            ),
            children: HashMap::new(),
            permissions: "-rwxr-xr-x".into(),
            owner: "root".into(),
            group: "root".into(),
            size: 35000 + (name.len() * 1000), // Fake realistic size
            is_executable: true,
            is_critical: critical,
        }
    }
    pub fn symlink(name: &str, target: &str) -> Self {
        Inode {
            name: name.into(),
            is_dir: false,
            data: target.into(),
            children: HashMap::new(),
            permissions: "lrwxrwxrwx".into(),
            owner: "root".into(),
            group: "root".into(),
            size: target.len(),
            is_executable: false,
            is_critical: false,
        }
    }
}

#[derive(Clone)]
pub struct VfsHandle {
    pub path: String,
    pub offset: usize,
    pub writable: bool,
}

pub struct Vfs {
    root: Inode,
    pub cwd: String,
    handles: HashMap<u32, VfsHandle>,
    next_handle: u32,
    pub kernel_panic: bool,
    pub panic_reason: String,
    default_owner: String,
    default_group: String,
    ignore_critical_deletes: bool,
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}

impl Vfs {
    pub fn new() -> Self {
        Vfs {
            root: Inode::dir("/"),
            cwd: "/".into(),
            handles: HashMap::new(),
            next_handle: 1,
            kernel_panic: false,
            panic_reason: String::new(),
            default_owner: "user".into(),
            default_group: "user".into(),
            ignore_critical_deletes: false,
        }
    }

    pub fn init(&mut self) {
        // Create main directories
        for d in [
            "bin", "sbin", "dev", "etc", "home", "lib", "lib64", "proc", "sys", "tmp", "usr",
            "var", "boot", "root", "opt", "mnt", "media", "run",
        ]
        .iter()
        {
            self.root.children.insert((*d).into(), Inode::dir(d));
        }

        // Populate /bin with real binaries
        if let Some(bin) = self.root.children.get_mut("bin") {
            // Critical binaries - deleting these causes kernel panic
            bin.children
                .insert("sh".into(), Inode::binary("sh", "Bourne shell", true));
            bin.children.insert(
                "bash".into(),
                Inode::binary("bash", "Bourne Again SHell", true),
            );
            bin.children.insert(
                "dash".into(),
                Inode::binary("dash", "Debian Almquist Shell", false),
            );

            // Core utilities
            bin.children.insert(
                "ls".into(),
                Inode::binary("ls", "list directory contents", false),
            );
            bin.children.insert(
                "cat".into(),
                Inode::binary("cat", "concatenate files", false),
            );
            bin.children
                .insert("cp".into(), Inode::binary("cp", "copy files", false));
            bin.children
                .insert("mv".into(), Inode::binary("mv", "move files", false));
            bin.children
                .insert("rm".into(), Inode::binary("rm", "remove files", false));
            bin.children.insert(
                "mkdir".into(),
                Inode::binary("mkdir", "make directories", false),
            );
            bin.children.insert(
                "rmdir".into(),
                Inode::binary("rmdir", "remove directories", false),
            );
            bin.children.insert(
                "touch".into(),
                Inode::binary("touch", "change file timestamps", false),
            );
            bin.children.insert(
                "chmod".into(),
                Inode::binary("chmod", "change file permissions", false),
            );
            bin.children.insert(
                "chown".into(),
                Inode::binary("chown", "change file owner", false),
            );
            bin.children.insert(
                "pwd".into(),
                Inode::binary("pwd", "print working directory", false),
            );
            bin.children
                .insert("echo".into(), Inode::binary("echo", "display text", false));
            bin.children.insert(
                "date".into(),
                Inode::binary("date", "print date and time", false),
            );
            bin.children.insert(
                "uname".into(),
                Inode::binary("uname", "print system information", false),
            );
            bin.children.insert(
                "hostname".into(),
                Inode::binary("hostname", "show or set hostname", false),
            );
            bin.children.insert(
                "whoami".into(),
                Inode::binary("whoami", "print effective userid", false),
            );
            bin.children.insert(
                "id".into(),
                Inode::binary("id", "print user identity", false),
            );
            bin.children.insert(
                "groups".into(),
                Inode::binary("groups", "print group memberships", false),
            );
            bin.children.insert(
                "ps".into(),
                Inode::binary("ps", "report process status", false),
            );
            bin.children.insert(
                "kill".into(),
                Inode::binary("kill", "send signal to process", false),
            );
            bin.children.insert(
                "sleep".into(),
                Inode::binary("sleep", "delay execution", false),
            );
            bin.children.insert(
                "true".into(),
                Inode::binary("true", "do nothing, successfully", false),
            );
            bin.children.insert(
                "false".into(),
                Inode::binary("false", "do nothing, unsuccessfully", false),
            );
            bin.children.insert(
                "test".into(),
                Inode::binary("test", "check file types", false),
            );
            bin.children.insert(
                "expr".into(),
                Inode::binary("expr", "evaluate expressions", false),
            );
            bin.children.insert(
                "env".into(),
                Inode::binary("env", "run program in modified environment", false),
            );
            bin.children.insert(
                "printenv".into(),
                Inode::binary("printenv", "print environment", false),
            );
            bin.children.insert(
                "head".into(),
                Inode::binary("head", "output first part of files", false),
            );
            bin.children.insert(
                "tail".into(),
                Inode::binary("tail", "output last part of files", false),
            );
            bin.children.insert(
                "wc".into(),
                Inode::binary("wc", "word, line, byte count", false),
            );
            bin.children
                .insert("sort".into(), Inode::binary("sort", "sort lines", false));
            bin.children.insert(
                "uniq".into(),
                Inode::binary("uniq", "remove duplicate lines", false),
            );
            bin.children.insert(
                "grep".into(),
                Inode::binary("grep", "search for patterns", false),
            );
            bin.children
                .insert("sed".into(), Inode::binary("sed", "stream editor", false));
            bin.children.insert(
                "awk".into(),
                Inode::binary("awk", "pattern scanning", false),
            );
            bin.children.insert(
                "cut".into(),
                Inode::binary("cut", "remove sections from lines", false),
            );
            bin.children.insert(
                "tr".into(),
                Inode::binary("tr", "translate characters", false),
            );
            bin.children.insert(
                "tee".into(),
                Inode::binary("tee", "read from stdin and write to files", false),
            );
            bin.children.insert(
                "xargs".into(),
                Inode::binary("xargs", "build command lines", false),
            );
            bin.children.insert(
                "find".into(),
                Inode::binary("find", "search for files", false),
            );
            bin.children
                .insert("ln".into(), Inode::binary("ln", "make links", false));
            bin.children.insert(
                "readlink".into(),
                Inode::binary("readlink", "print symlink value", false),
            );
            bin.children
                .insert("df".into(), Inode::binary("df", "disk space usage", false));
            bin.children.insert(
                "du".into(),
                Inode::binary("du", "estimate file space usage", false),
            );
            bin.children.insert(
                "mount".into(),
                Inode::binary("mount", "mount filesystem", false),
            );
            bin.children.insert(
                "umount".into(),
                Inode::binary("umount", "unmount filesystem", false),
            );
            bin.children
                .insert("tar".into(), Inode::binary("tar", "tape archiver", false));
            bin.children.insert(
                "gzip".into(),
                Inode::binary("gzip", "compress files", false),
            );
            bin.children.insert(
                "gunzip".into(),
                Inode::binary("gunzip", "decompress files", false),
            );
            bin.children.insert(
                "more".into(),
                Inode::binary("more", "file perusal filter", false),
            );
            bin.children.insert(
                "less".into(),
                Inode::binary("less", "opposite of more", false),
            );
            bin.children.insert(
                "nano".into(),
                Inode::binary("nano", "simple text editor", false),
            );
            bin.children.insert(
                "vi".into(),
                Inode::binary("vi", "visual text editor", false),
            );
            bin.children
                .insert("vim".into(), Inode::binary("vim", "Vi IMproved", false));
            bin.children.insert(
                "clear".into(),
                Inode::binary("clear", "clear terminal screen", false),
            );
            bin.children.insert(
                "reset".into(),
                Inode::binary("reset", "reset terminal", false),
            );
            bin.children.insert(
                "stty".into(),
                Inode::binary("stty", "change terminal settings", false),
            );
            bin.children.insert(
                "dmesg".into(),
                Inode::binary("dmesg", "print kernel messages", false),
            );
            bin.children.insert(
                "login".into(),
                Inode::binary("login", "begin session", true),
            );
            bin.children
                .insert("su".into(), Inode::binary("su", "switch user", false));
            bin.children.insert(
                "sudo".into(),
                Inode::binary("sudo", "execute as superuser", false),
            );
            bin.children.insert(
                "passwd".into(),
                Inode::binary("passwd", "change password", false),
            );
        }

        // Populate /sbin with system binaries
        if let Some(sbin) = self.root.children.get_mut("sbin") {
            sbin.children.insert(
                "init".into(),
                Inode::binary("init", "process control initialization", true),
            );
            sbin.children.insert(
                "shutdown".into(),
                Inode::binary("shutdown", "halt the system", false),
            );
            sbin.children.insert(
                "reboot".into(),
                Inode::binary("reboot", "restart the system", false),
            );
            sbin.children.insert(
                "halt".into(),
                Inode::binary("halt", "stop the system", false),
            );
            sbin.children.insert(
                "poweroff".into(),
                Inode::binary("poweroff", "power off the system", false),
            );
            sbin.children.insert(
                "fsck".into(),
                Inode::binary("fsck", "check filesystem", false),
            );
            sbin.children.insert(
                "mkfs".into(),
                Inode::binary("mkfs", "build filesystem", false),
            );
            sbin.children.insert(
                "fdisk".into(),
                Inode::binary("fdisk", "partition table manipulator", false),
            );
            sbin.children.insert(
                "ip".into(),
                Inode::binary("ip", "show/manipulate routing", false),
            );
            sbin.children.insert(
                "ifconfig".into(),
                Inode::binary("ifconfig", "configure network interface", false),
            );
            sbin.children.insert(
                "route".into(),
                Inode::binary("route", "show/manipulate IP routing table", false),
            );
            sbin.children.insert(
                "iptables".into(),
                Inode::binary("iptables", "IP packet filter", false),
            );
            sbin.children.insert(
                "modprobe".into(),
                Inode::binary("modprobe", "add/remove kernel modules", false),
            );
            sbin.children.insert(
                "insmod".into(),
                Inode::binary("insmod", "insert kernel module", false),
            );
            sbin.children.insert(
                "rmmod".into(),
                Inode::binary("rmmod", "remove kernel module", false),
            );
            sbin.children.insert(
                "lsmod".into(),
                Inode::binary("lsmod", "list kernel modules", false),
            );
            sbin.children.insert(
                "getty".into(),
                Inode::binary("getty", "set terminal mode", true),
            );
            sbin.children.insert(
                "agetty".into(),
                Inode::binary("agetty", "alternative getty", false),
            );
        }

        // Populate /dev with device files
        if let Some(dev) = self.root.children.get_mut("dev") {
            dev.children.insert("null".into(), Inode::file("null", ""));
            dev.children.insert("zero".into(), Inode::file("zero", ""));
            dev.children
                .insert("random".into(), Inode::file("random", ""));
            dev.children
                .insert("urandom".into(), Inode::file("urandom", ""));
            dev.children.insert("tty".into(), Inode::file("tty", ""));
            dev.children.insert("tty0".into(), Inode::file("tty0", ""));
            dev.children.insert("tty1".into(), Inode::file("tty1", ""));
            dev.children
                .insert("console".into(), Inode::file("console", ""));
            dev.children
                .insert("stdin".into(), Inode::symlink("stdin", "/proc/self/fd/0"));
            dev.children
                .insert("stdout".into(), Inode::symlink("stdout", "/proc/self/fd/1"));
            dev.children
                .insert("stderr".into(), Inode::symlink("stderr", "/proc/self/fd/2"));
            dev.children
                .insert("fd".into(), Inode::symlink("fd", "/proc/self/fd"));
            dev.children.insert("sda".into(), Inode::file("sda", ""));
            dev.children.insert("sda1".into(), Inode::file("sda1", ""));
        }

        // Populate /etc with config files
        if let Some(etc) = self.root.children.get_mut("etc") {
            etc.children
                .insert("hostname".into(), Inode::file("hostname", "kpawnd"));
            etc.children.insert(
                "hosts".into(),
                Inode::file(
                    "hosts",
                    "127.0.0.1\tlocalhost\n::1\t\tlocalhost\n127.0.1.1\tkpawnd\n",
                ),
            );
            etc.children.insert(
                "resolv.conf".into(),
                Inode::file("resolv.conf", "nameserver 8.8.8.8\nnameserver 8.8.4.4\n"),
            );
            etc.children.insert("passwd".into(), Inode::file("passwd", "root:x:0:0:root:/root:/bin/bash\nuser:x:1000:1000:User:/home/user:/bin/bash\nnobody:x:65534:65534:Nobody:/:/usr/sbin/nologin\n"));
            etc.children.insert(
                "shadow".into(),
                Inode::file(
                    "shadow",
                    "root:!:19000:0:99999:7:::\nuser:!:19000:0:99999:7:::\n",
                ),
            );
            etc.children.insert(
                "group".into(),
                Inode::file("group", "root:x:0:\nuser:x:1000:user\nnogroup:x:65534:\n"),
            );
            etc.children.insert("fstab".into(), Inode::file("fstab", "# /etc/fstab: static file system information.\n/dev/sda1\t/\text4\tdefaults\t0\t1\n"));
            etc.children.insert("motd".into(), Inode::file("motd", "Welcome to kpawnd GNU/Linux!\n\nType 'help' for available commands.\nType 'echo github' to visit the project page.\n"));
            etc.children.insert(
                "issue".into(),
                Inode::file("issue", "kpawnd GNU/Linux \\n \\l\n\n"),
            );
            etc.children.insert("os-release".into(), Inode::file("os-release", "NAME=\"kpawnd GNU/Linux\"\nVERSION=\"0.2.0\"\nID=kpawnd\nVERSION_ID=\"0.2.0\"\nPRETTY_NAME=\"kpawnd GNU/Linux 0.2.0\"\nHOME_URL=\"https://github.com/kpawnd\"\n"));
            etc.children.insert(
                "shells".into(),
                Inode::file("shells", "/bin/sh\n/bin/bash\n/bin/dash\n"),
            );
            etc.children.insert("profile".into(), Inode::file("profile", "# /etc/profile: system-wide .profile file\nexport PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\nexport PS1='\\u@\\h:\\w\\$ '\n"));
            etc.children.insert(
                "github".into(),
                Inode::file("github", "https://github.com/kpawnd"),
            );
        }

        // Populate /proc with process info
        if let Some(proc_dir) = self.root.children.get_mut("proc") {
            proc_dir.children.insert(
                "version".into(),
                Inode::file(
                    "version",
                    "Linux version 6.1.0-kpawnd (gcc version 12.2.0) #1 SMP PREEMPT_DYNAMIC\n",
                ),
            );
            proc_dir.children.insert("cpuinfo".into(), Inode::file("cpuinfo", "processor\t: 0\nvendor_id\t: WebAssembly\nmodel name\t: WASM Virtual CPU\ncpu MHz\t\t: 1000.000\ncache size\t: 256 KB\n"));
            proc_dir.children.insert("meminfo".into(), Inode::file("meminfo", "MemTotal:         128 kB\nMemFree:          100 kB\nBuffers:            8 kB\nCached:            16 kB\n"));
            proc_dir
                .children
                .insert("uptime".into(), Inode::file("uptime", "0.00 0.00\n"));
            proc_dir.children.insert(
                "loadavg".into(),
                Inode::file("loadavg", "0.00 0.00 0.00 1/1 1\n"),
            );
            proc_dir.children.insert(
                "filesystems".into(),
                Inode::file("filesystems", "nodev\tproc\nnodev\ttmpfs\n\text4\n"),
            );
            proc_dir.children.insert("mounts".into(), Inode::file("mounts", "/dev/sda1 / ext4 rw,relatime 0 0\nproc /proc proc rw,nosuid,nodev,noexec 0 0\nsysfs /sys sysfs rw,nosuid,nodev,noexec 0 0\n"));

            let mut self_dir = Inode::dir("self");
            self_dir
                .children
                .insert("cmdline".into(), Inode::file("cmdline", "/bin/bash\x00"));
            self_dir.children.insert(
                "status".into(),
                Inode::file(
                    "status",
                    "Name:\tbash\nPid:\t1\nUid:\t1000\t1000\t1000\t1000\n",
                ),
            );
            proc_dir.children.insert("self".into(), self_dir);
        }

        // Populate /home/user
        if let Some(home) = self.root.children.get_mut("home") {
            let mut user = Inode::dir("user");
            user.permissions = "drwxr-xr-x".into();
            user.owner = "user".into();
            user.group = "user".into();
            user.children.insert(".bashrc".into(), Inode::file(".bashrc", "# ~/.bashrc: executed by bash for non-login shells.\n\nalias ll='ls -la'\nalias la='ls -A'\n\nPS1='\\u@\\h:\\w\\$ '\n"));
            user.children.insert(".profile".into(), Inode::file(".profile", "# ~/.profile: executed by the command interpreter for login shells\n\nif [ -f ~/.bashrc ]; then\n    . ~/.bashrc\nfi\n"));
            user.children.insert("readme.txt".into(), Inode::file("readme.txt", "This is a terminal emulator running in your browser.\nType 'echo github' to visit the project page.\n\nTry these commands:\n  neofetch  - Display system info\n  ls /bin   - List available commands\n  nano      - Edit files\n  python    - Python REPL\n  doom      - Play a game!\n"));

            let mut documents = Inode::dir("Documents");
            documents.owner = "user".into();
            documents.group = "user".into();
            documents.children.insert(
                "notes.txt".into(),
                Inode::file("notes.txt", "My notes file\n"),
            );
            user.children.insert("Documents".into(), documents);

            let mut downloads = Inode::dir("Downloads");
            downloads.owner = "user".into();
            downloads.group = "user".into();
            user.children.insert("Downloads".into(), downloads);

            home.children.insert("user".into(), user);
        }

        // Populate /var
        if let Some(var) = self.root.children.get_mut("var") {
            var.children.insert("log".into(), Inode::dir("log"));
            var.children.insert("tmp".into(), Inode::dir("tmp"));
            var.children.insert("run".into(), Inode::dir("run"));
            var.children.insert("cache".into(), Inode::dir("cache"));

            if let Some(log) = var.children.get_mut("log") {
                log.children
                    .insert("syslog".into(), Inode::file("syslog", ""));
                log.children
                    .insert("auth.log".into(), Inode::file("auth.log", ""));
                log.children
                    .insert("kern.log".into(), Inode::file("kern.log", ""));
            }
        }

        // Populate /usr
        if let Some(usr) = self.root.children.get_mut("usr") {
            usr.children.insert("bin".into(), Inode::dir("bin"));
            usr.children.insert("sbin".into(), Inode::dir("sbin"));
            usr.children.insert("lib".into(), Inode::dir("lib"));
            usr.children.insert("share".into(), Inode::dir("share"));
            usr.children.insert("local".into(), Inode::dir("local"));

            if let Some(share) = usr.children.get_mut("share") {
                share.children.insert("man".into(), Inode::dir("man"));
                share.children.insert("doc".into(), Inode::dir("doc"));
            }
        }

        // Populate /boot
        if let Some(boot) = self.root.children.get_mut("boot") {
            boot.children.insert(
                "vmlinuz-6.1.0-kpawnd".into(),
                Inode::file("vmlinuz-6.1.0-kpawnd", "[kernel image]"),
            );
            boot.children.insert(
                "initrd.img-6.1.0-kpawnd".into(),
                Inode::file("initrd.img-6.1.0-kpawnd", "[initramfs]"),
            );
            boot.children.insert("grub".into(), Inode::dir("grub"));

            if let Some(grub) = boot.children.get_mut("grub") {
                grub.children.insert("grub.cfg".into(), Inode::file("grub.cfg", "# GRUB configuration file\nset default=0\nset timeout=5\n\nmenuentry 'kpawnd GNU/Linux' {\n    linux /boot/vmlinuz-6.1.0-kpawnd root=/dev/sda1 ro quiet\n    initrd /boot/initrd.img-6.1.0-kpawnd\n}\n"));
            }
        }

        // Populate /lib with libraries
        if let Some(lib) = self.root.children.get_mut("lib") {
            lib.children
                .insert("x86_64-linux-gnu".into(), Inode::dir("x86_64-linux-gnu"));
            lib.children.insert("modules".into(), Inode::dir("modules"));
            lib.children
                .insert("firmware".into(), Inode::dir("firmware"));
        }

        // Populate /root with some files for root user
        if let Some(root_home) = self.root.children.get_mut("root") {
            root_home.children.insert(
                ".bashrc".into(),
                Inode::file(".bashrc", "# Root's bashrc\nPS1='\\u@\\h:\\w\\$ '\nalias ls='ls --color=auto'\nalias ll='ls -la'\n"),
            );
            root_home.children.insert(
                ".profile".into(),
                Inode::file(".profile", "# Root's profile\nexport PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\n"),
            );
            root_home.children.insert(
                ".vimrc".into(),
                Inode::file(".vimrc", "\" Vim configuration for root\nsyntax on\nset number\n"),
            );
            root_home.children.insert(
                "README".into(),
                Inode::file("README", "Welcome to root's home directory.\n\nBe careful with administrative commands.\nAlways double-check before running destructive operations.\n"),
            );
        }
    }
    pub fn normalize(&self, path: &str) -> String {
        let raw = if path.starts_with('/') {
            path.to_string()
        } else {
            let base = self.cwd.trim_end_matches('/');
            format!("{}/{}", base, path)
        };
        // Process . and .. components
        let mut parts: Vec<&str> = Vec::new();
        for part in raw.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                _ => parts.push(part),
            }
        }
        if parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", parts.join("/"))
        }
    }
    pub fn resolve(&self, path: &str) -> Option<&Inode> {
        let norm = self.normalize(path);
        let mut node = &self.root;
        for part in norm.split('/').filter(|s| !s.is_empty()) {
            node = node.children.get(part)?;
        }
        Some(node)
    }
    pub fn resolve_mut(&mut self, path: &str) -> Option<&mut Inode> {
        let norm = self.normalize(path);
        let mut node = &mut self.root;
        for part in norm.split('/').filter(|s| !s.is_empty()) {
            node = node.children.get_mut(part)?;
        }
        Some(node)
    }
    pub fn cd(&mut self, path: &str) -> Result<(), &'static str> {
        let target = if path == ".." {
            let mut parts: Vec<_> = self.cwd.split('/').filter(|s| !s.is_empty()).collect();
            parts.pop();
            if parts.is_empty() {
                "/".into()
            } else {
                format!("/{}", parts.join("/"))
            }
        } else {
            self.normalize(path)
        };
        match self.resolve(&target) {
            Some(node) if node.is_dir => {
                self.cwd = if target == "/" {
                    "/".into()
                } else {
                    target.trim_end_matches('/').into()
                };
                Ok(())
            }
            Some(_) => Err("not a directory"),
            None => Err("no such directory"),
        }
    }
    pub fn open(&mut self, path: &str, write: bool) -> Result<u32, &'static str> {
        if let Some(node) = self.resolve(path) {
            if node.is_dir {
                return Err("is directory");
            }
            let h = self.next_handle;
            self.next_handle += 1;
            self.handles.insert(
                h,
                VfsHandle {
                    path: self.normalize(path),
                    offset: 0,
                    writable: write,
                },
            );
            Ok(h)
        } else {
            Err("no such file")
        }
    }
    pub fn read(&mut self, handle: u32, size: usize) -> Result<String, &'static str> {
        let (path, offset) = {
            let h = self.handles.get(&handle).ok_or("bad handle")?;
            (h.path.clone(), h.offset)
        };
        let inode = self.resolve(&path).ok_or("gone")?;
        let start = offset;
        let end = (start + size).min(inode.data.len());
        let out = inode.data[start..end].to_string();
        if let Some(h) = self.handles.get_mut(&handle) {
            h.offset = end;
        }
        Ok(out)
    }
    pub fn write(&mut self, handle: u32, data: &str) -> Result<(), &'static str> {
        let (path, writable) = {
            let h = self.handles.get(&handle).ok_or("bad handle")?;
            (h.path.clone(), h.writable)
        };
        if !writable {
            return Err("not writable");
        }
        let new_len = if let Some(inode) = self.resolve_mut(&path) {
            inode.data.push_str(data);
            inode.data.len()
        } else {
            return Err("gone");
        };
        if let Some(h) = self.handles.get_mut(&handle) {
            h.offset = new_len;
        }
        Ok(())
    }
    pub fn close(&mut self, handle: u32) {
        self.handles.remove(&handle);
    }

    /// Check if a file is critical (deleting it should cause a panic)
    pub fn is_critical(&self, path: &str) -> bool {
        if let Some(node) = self.resolve(path) {
            return node.is_critical;
        }
        false
    }

    /// Remove a file or directory, returns error if critical
    pub fn remove(&mut self, path: &str) -> Result<(), String> {
        let norm = self.normalize(path);

        // Check if it's a critical file
        if self.is_critical(&norm) && !self.ignore_critical_deletes {
            let filename = norm.split('/').next_back().unwrap_or("unknown");
            self.kernel_panic = true;
            self.panic_reason = format!(
                "KERNEL PANIC - not syncing: Attempted to remove critical system binary: {}\n\
                 \n\
                 CPU: 0 PID: 1 Comm: rm Not tainted 6.1.0-kpawnd #1\n\
                 Hardware name: WASM Virtual Machine\n\
                 Call Trace:\n\
                  <TASK>\n\
                  vfs_unlink+0x1a/0x30\n\
                  do_unlinkat+0x2c/0x40\n\
                  __x64_sys_unlink+0x1c/0x30\n\
                  do_syscall_64+0x5c/0x90\n\
                  </TASK>\n\
                 \n\
                 Kernel Offset: 0x0 from 0xffffffff81000000\n\
                 ---[ end Kernel panic - not syncing: {} ]---",
                filename, filename
            );
            return Err(format!(
                "KERNEL PANIC: Cannot remove critical system file '{}'",
                filename
            ));
        }

        // Get parent path and filename
        let parts: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err("cannot remove root".into());
        }

        let filename = parts.last().unwrap().to_string();
        let parent_path = if parts.len() == 1 {
            "/".to_string()
        } else {
            format!("/{}", parts[..parts.len() - 1].join("/"))
        };

        // Check if target exists and get its properties
        let is_dir = match self.resolve(&norm) {
            Some(node) => node.is_dir,
            None => return Err("no such file or directory".into()),
        };

        // Remove from parent
        if let Some(parent) = self.resolve_mut(&parent_path) {
            if is_dir {
                if let Some(node) = parent.children.get(&filename) {
                    if !node.children.is_empty() {
                        return Err("directory not empty".into());
                    }
                }
            }
            parent.children.remove(&filename);
            Ok(())
        } else {
            Err("parent directory not found".into())
        }
    }

    /// Recursively remove a file or directory tree. Will error on critical binaries.
    pub fn remove_recursive(&mut self, path: &str) -> Result<(), String> {
        let norm = self.normalize(path);
        // If target doesn't exist, return error
        let node = match self.resolve(&norm) {
            Some(n) => n.clone(),
            None => return Err("no such file or directory".into()),
        };

        if node.is_dir {
            // Collect child paths to avoid borrow issues
            let mut child_paths: Vec<String> = Vec::new();
            if let Some(current) = self.resolve(&norm) {
                for (name, _) in &current.children {
                    let child = if norm == "/" { format!("/{}", name) } else { format!("{}/{}", norm, name) };
                    child_paths.push(child);
                }
            }
            for child in child_paths {
                self.remove_recursive(&child)?;
            }
        }
        // Finally remove the empty directory or file
        self.remove(&norm)
    }

    /// Create a new file
    pub fn create_file(&mut self, path: &str, data: &str) -> Result<(), &'static str> {
        let norm = self.normalize(path);
        let parts: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err("invalid path");
        }

        let filename = parts.last().unwrap().to_string();
        let parent_path = if parts.len() == 1 {
            "/".to_string()
        } else {
            format!("/{}", parts[..parts.len() - 1].join("/"))
        };

        let owner = self.default_owner.clone();
        let group = self.default_group.clone();
        if let Some(parent) = self.resolve_mut(&parent_path) {
            if !parent.is_dir {
                return Err("parent is not a directory");
            }
            let mut new_file = Inode::file(&filename, data);
            new_file.owner = owner;
            new_file.group = group;
            parent.children.insert(filename, new_file);
            Ok(())
        } else {
            Err("parent directory not found")
        }
    }

    /// Create a directory
    pub fn create_dir(&mut self, path: &str) -> Result<(), &'static str> {
        let norm = self.normalize(path);
        let parts: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err("invalid path");
        }

        let dirname = parts.last().unwrap().to_string();
        let parent_path = if parts.len() == 1 {
            "/".to_string()
        } else {
            format!("/{}", parts[..parts.len() - 1].join("/"))
        };

        let owner = self.default_owner.clone();
        let group = self.default_group.clone();
        if let Some(parent) = self.resolve_mut(&parent_path) {
            if !parent.is_dir {
                return Err("parent is not a directory");
            }
            if parent.children.contains_key(&dirname) {
                return Err("already exists");
            }
            let mut new_dir = Inode::dir(&dirname);
            new_dir.owner = owner;
            new_dir.group = group;
            parent.children.insert(dirname, new_dir);
            Ok(())
        } else {
            Err("parent directory not found")
        }
    }

    /// Update file contents
    pub fn write_file(&mut self, path: &str, data: &str) -> Result<(), &'static str> {
        if let Some(node) = self.resolve_mut(path) {
            if node.is_dir {
                return Err("is a directory");
            }
            node.data = data.into();
            node.size = data.len();
            Ok(())
        } else {
            Err("no such file")
        }
    }

    /// List directory contents with details
    pub fn list_detailed(&self, path: &str) -> Result<Vec<String>, &'static str> {
        if let Some(node) = self.resolve(path) {
            if !node.is_dir {
                return Err("not a directory");
            }

            let mut entries: Vec<_> = node.children.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));

            let output: Vec<String> = entries
                .iter()
                .map(|(name, child)| {
                    let name_display = if child.is_dir {
                        format!("\x1b[COLOR:blue]{}\x1b[COLOR:reset]", name)
                    } else if child.is_executable {
                        format!("\x1b[COLOR:green]{}\x1b[COLOR:reset]", name)
                    } else {
                        name.to_string()
                    };

                    format!(
                        "{} {:>3} {:>8} {:>8} {:>8} {} {}",
                        child.permissions,
                        1,
                        child.owner,
                        child.group,
                        child.size,
                        "Nov 29 12:00",
                        name_display
                    )
                })
                .collect();

            Ok(output)
        } else {
            Err("no such directory")
        }
    }

    pub fn set_default_owner(&mut self, owner: &str, group: &str) {
        self.default_owner = owner.into();
        self.default_group = group.into();
    }

    pub fn get_default_owner(&self) -> String {
        self.default_owner.clone()
    }

    pub fn get_default_group(&self) -> String {
        self.default_group.clone()
    }

    pub fn set_ignore_critical_deletes(&mut self, val: bool) {
        self.ignore_critical_deletes = val;
    }

    /// Get all user-created files for persistence
    /// Returns a JSON string of path -> content mapping
    pub fn export_user_files(&self) -> String {
        let mut files: HashMap<String, String> = HashMap::new();

        // Collect all non-system files from the entire filesystem
        self.collect_user_files_recursive(&self.root, "", &mut files);

        serde_json::to_string(&files).unwrap_or_else(|_| "{}".to_string())
    }

    #[allow(clippy::only_used_in_recursion)]
    fn collect_user_files_recursive(
        &self,
        node: &Inode,
        path: &str,
        files: &mut HashMap<String, String>,
    ) {
        for (name, child) in &node.children {
            let child_path = if path.is_empty() {
                format!("/{}", name)
            } else {
                format!("{}/{}", path, name)
            };

            if child.is_dir {
                // Skip system directories that shouldn't be persisted
                if !matches!(
                    name.as_str(),
                    "bin"
                        | "sbin"
                        | "dev"
                        | "proc"
                        | "sys"
                        | "boot"
                        | "lib"
                        | "lib64"
                        | "usr"
                        | "opt"
                ) {
                    self.collect_user_files_recursive(child, &child_path, files);
                }
            } else if !child.is_executable && !child.is_critical {
                // Save user files (non-executable, non-critical)
                // Skip system config files
                if !child_path.starts_with("/etc/") || child_path.starts_with("/etc/user/") {
                    files.insert(child_path, child.data.clone());
                }
            }
        }
    }

    /// Import user files from JSON string
    pub fn import_user_files(&mut self, json: &str) {
        if let Ok(files) = serde_json::from_str::<HashMap<String, String>>(json) {
            for (path, content) in files {
                // Create parent directories if needed
                if let Some(parent_end) = path.rfind('/') {
                    let parent = &path[..parent_end];
                    if !parent.is_empty() {
                        let _ = self.mkdir_p(parent);
                    }
                }

                // Create or update the file
                if self.resolve(&path).is_some() {
                    let _ = self.write_file(&path, &content);
                } else {
                    let _ = self.create_file(&path, &content);
                }
            }
        }
    }

    /// Recursively create directories
    fn mkdir_p(&mut self, path: &str) -> Result<(), &'static str> {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let mut current = String::new();

        for part in parts {
            current = format!("{}/{}", current, part);
            if self.resolve(&current).is_none() {
                self.create_dir(&current)?;
            }
        }
        Ok(())
    }
}
