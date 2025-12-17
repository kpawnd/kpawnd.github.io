use crate::{memory::Memory, process::ProcessTable, process::Scheduler, vfs::Vfs};

pub const VERSION: &str = "0.6.7";
pub const TOTAL_MEM: u32 = 33554432; // 32MB
pub const KERNEL_VERSION: &str = "6.7.0-kpawnd";

#[derive(PartialEq, Clone, Copy)]
pub enum KernelState {
    Off,
    Bios,
    Boot,
    Init,
    Run,
    Halt,
}

pub struct Kernel {
    pub state: KernelState,
    pub mem: Memory,
    pub proc: ProcessTable,
    pub fs: Vfs,
    pub ticks: u64,
    log: Vec<String>,
    boot_index: usize,
    pub scheduler: Scheduler,
    pub memory_panic: bool,
    pub memory_panic_reason: String,
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

impl Kernel {
    pub fn new() -> Self {
        Kernel {
            state: KernelState::Off,
            mem: Memory::new(TOTAL_MEM),
            proc: ProcessTable::new(),
            fs: Vfs::new(),
            ticks: 0,
            log: Vec::new(),
            boot_index: 0,
            scheduler: Scheduler::new(),
            memory_panic: false,
            memory_panic_reason: String::new(),
        }
    }
    fn klog(&mut self, msg: &str) {
        let ts = self.ticks as f64 * 0.000001;
        self.log.push(format!("[{:12.6}] {}", ts, msg));
    }
    fn raw_log(&mut self, msg: &str) {
        self.log.push(msg.to_string());
    }
    fn memory_panic(&mut self, reason: &str) {
        self.memory_panic = true;
        self.memory_panic_reason = format!(
            "KERNEL PANIC - not syncing: Out of memory: {}\n\
             \n\
             CPU: 0 PID: 1 Comm: kernel Not tainted 6.1.0-kpawnd #1\n\
             Hardware name: WASM Virtual Machine\n\
             Call Trace:\n\
              <TASK>\n\
              __alloc_pages+0x1a/0x30\n\
              alloc_pages+0x2c/0x40\n\
              __get_free_pages+0x1c/0x30\n\
              </TASK>\n\
             \n\
             Memory: {}K/{}K available\n\
             ---[ end Kernel panic - not syncing: {} ]---",
            reason,
            self.mem.free / 1024,
            self.mem.total / 1024,
            reason
        );
    }
    pub fn generate_boot_log(&mut self) {
        if !self.log.is_empty() {
            return;
        }

        self.state = KernelState::Bios;
        self.ticks = 0;

        self.raw_log("SeaBIOS (version 1.16.2-debian-1.16.2-1)");
        self.raw_log("");
        self.raw_log("iPXE (https://ipxe.org) 00:03.0 CA00 PCI2.10 PnP PMM+07F91410+07EF1410 CA00");
        self.raw_log("");
        self.raw_log("");
        self.raw_log("Booting from Hard Disk...");

        self.ticks = 100;

        self.state = KernelState::Boot;

        self.raw_log("");
        self.klog(&format!(
            "Linux version {} (gcc version 12.2.0) #1 SMP PREEMPT_DYNAMIC",
            KERNEL_VERSION
        ));
        self.ticks += 10;
        self.klog("Command line: BOOT_IMAGE=/boot/vmlinuz-kpawnd root=/dev/wasm0 ro quiet");
        self.ticks += 5;
        self.klog("x86/fpu: x87 FPU on board");
        self.ticks += 2;
        self.klog("BIOS-provided physical RAM map:");
        self.ticks += 1;
        self.klog(&format!(
            "BIOS-e820: [mem 0x0000000000000000-0x{:016x}] usable",
            self.mem.total as u64 * 1024
        ));
        self.ticks += 5;
        self.klog("NX (Execute Disable) protection: active");
        self.ticks += 2;
        self.klog("SMBIOS 2.8 present");
        self.ticks += 3;
        self.klog("DMI: WASM Virtual Machine, BIOS 1.16.2 04/01/2014");
        self.ticks += 5;
        self.klog("Hypervisor detected: Browser/WASM");
        self.ticks += 10;

        self.klog(&format!(
            "Memory: {}K/{}K available (kernel code, reserved, data)",
            (self.mem.total * 90) / 100 / 1024,
            self.mem.total / 1024
        ));
        self.ticks += 5;
        if self.mem.alloc(4096).is_none() {
            self.memory_panic("Failed to allocate kernel memory during boot");
            return;
        }
        self.klog(&format!(
            "Zone ranges: Normal [mem 0x00000000-0x{:08x}]",
            self.mem.total
        ));
        self.ticks += 3;
        self.klog("Movable zone start for each node");
        self.ticks += 2;
        self.klog(&format!(
            "Early memory node ranges: node 0: [mem 0x00000000-0x{:08x}]",
            self.mem.total
        ));
        self.ticks += 5;
        self.klog("Initmem setup node 0 [mem 0x00000000-0x0001ffff]");
        self.ticks += 10;

        // ============ CPU Init ============
        self.klog("smpboot: Allowing 1 CPUs, 0 hotplug CPUs");
        self.ticks += 5;
        self.klog("smpboot: CPU0: WebAssembly Virtual CPU @ 1.0GHz (family: 0x6, model: 0x1, stepping: 0x0)");
        self.ticks += 3;
        self.klog("Performance Events: unsupported p6 CPU model 1 no PMU driver");
        self.ticks += 5;
        self.klog("rcu: Hierarchical SRCU implementation.");
        self.ticks += 2;
        self.klog("smp: Bringing up secondary CPUs ...");
        self.ticks += 3;
        self.klog("smp: Brought up 1 node, 1 CPU");
        self.ticks += 10;

        // ============ Kernel Subsystems ============
        self.state = KernelState::Init;

        self.klog("devtmpfs: initialized");
        self.ticks += 5;
        self.klog("clocksource: jiffies: mask: 0xffffffff max_cycles: 0xffffffff");
        self.ticks += 3;
        self.klog("NET: Registered PF_NETLINK/PF_ROUTE protocol family");
        self.ticks += 5;
        self.klog("DMA: preallocated 128 KiB GFP_KERNEL pool for atomic allocations");
        self.ticks += 3;
        self.klog("thermal_sys: Registered thermal governor 'fair_share'");
        self.ticks += 5;
        self.klog("PCI: Using configuration type 1 for base access");
        self.ticks += 10;

        // ============ VFS Init ============
        self.fs.init();
        self.klog("VFS: Disk quotas dquot_6.6.0");
        self.ticks += 3;
        self.klog("VFS: Dquot-cache hash table entries: 512");
        self.ticks += 5;
        self.klog("Mounting rootfs on wasm0...");
        self.ticks += 10;
        self.klog("EXT4-fs (wasm0): mounted filesystem with ordered data mode");
        self.ticks += 5;
        self.klog("VFS: Mounted root (ext4 filesystem) readonly on device 8:0.");
        self.ticks += 3;
        self.klog("devtmpfs: mounted");
        self.ticks += 5;

        // ============ Device Detection ============
        self.klog("input: Power Button as /devices/LNXSYSTM:00/LNXPWRBN:00/input/input0");
        self.ticks += 3;
        self.klog("serial8250: ttyS0 at I/O 0x3f8 (irq = 4) is a 16550A");
        self.ticks += 5;
        self.klog("tty: tty0: 80x25 VGA console");
        self.ticks += 3;
        self.klog("virtio_net virtio0 eth0: Validate memory region 0x0000-0xffff");
        self.ticks += 5;
        self.klog("virtio_blk virtio1: [wasm0] 262144 sectors (128 MiB)");
        self.ticks += 10;

        // ============ Network Init ============
        self.klog("NET: Registered PF_INET protocol family");
        self.ticks += 3;
        self.klog("NET: Registered PF_INET6 protocol family");
        self.ticks += 3;
        self.klog("Segment Routing with IPv6");
        self.ticks += 5;

        // ============ Systemd Init ============
        let init_pid = match self.proc.spawn("init", 0, &mut self.mem) {
            Some(pid) => pid,
            None => {
                self.memory_panic("Failed to allocate memory for init process");
                return;
            }
        };
        self.klog(&format!(
            "Run /sbin/init as init process (pid {})",
            init_pid
        ));
        self.ticks += 10;
        self.raw_log("");
        self.raw_log("         systemd 252 running in system mode (+PAM +AUDIT +SELINUX)");
        self.raw_log("         Detected virtualization wasm.");
        self.raw_log("         Detected architecture wasm32.");
        self.raw_log("");
        self.ticks += 20;

        // ============ Systemd Services ============
        self.raw_log("[  OK  ] Created slice Slice /system/getty.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Created slice User and Session Slice.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Started Forward Password Requests to Wall...");
        self.ticks += 5;
        self.raw_log("[  OK  ] Reached target Local Encrypted Volumes.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Reached target Network (Pre).");
        self.ticks += 5;
        self.raw_log("[  OK  ] Reached target Path Units.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Reached target Slice Units.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Reached target Swaps.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Listening on Process Core Dump Socket.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Listening on Journal Socket (/dev/log).");
        self.ticks += 3;
        self.raw_log("[  OK  ] Listening on Journal Socket.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Listening on udev Control Socket.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Listening on udev Kernel Socket.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Mounted Huge Pages File System.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Mounted POSIX Message Queue File System.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Mounted Kernel Debug File System.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Mounted Kernel Trace File System.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Starting Load Kernel Module configfs...");
        self.ticks += 3;
        self.raw_log("[  OK  ] Starting Load Kernel Module drm...");
        self.ticks += 5;
        self.raw_log("[  OK  ] Starting Load Kernel Module fuse...");
        self.ticks += 3;
        self.raw_log("[  OK  ] Starting Journal Service...");
        self.ticks += 10;
        self.raw_log("[  OK  ] Started Journal Service.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Starting Flush Journal to Persistent Storage...");
        self.ticks += 3;
        self.raw_log("[  OK  ] Finished Flush Journal to Persistent Storage.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Starting Create Volatile Files and Directories...");
        self.ticks += 3;
        self.raw_log("[  OK  ] Finished Create Volatile Files and Directories.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Starting Network Manager...");
        self.ticks += 10;
        self.raw_log("[  OK  ] Started Network Manager.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Reached target Network.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Reached target Network is Online.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Starting Permit User Sessions...");
        self.ticks += 3;
        self.raw_log("[  OK  ] Finished Permit User Sessions.");
        self.ticks += 5;
        self.raw_log("[  OK  ] Started Getty on tty1.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Reached target Login Prompts.");
        self.ticks += 5;

        let sh_pid = match self.proc.spawn("sh", init_pid, &mut self.mem) {
            Some(pid) => pid,
            None => {
                self.memory_panic("Failed to allocate memory for shell process");
                return;
            }
        };
        self.scheduler.add(sh_pid, crate::process::Priority::Normal);
        self.ticks += 10;
        self.raw_log("[  OK  ] Reached target Multi-User System.");
        self.ticks += 3;
        self.raw_log("[  OK  ] Reached target Graphical Interface.");
        self.ticks += 5;
        self.raw_log("");

        self.state = KernelState::Run;
        self.raw_log(&format!(
            "kpawnd v{} login: user (automatic login)",
            VERSION
        ));
        self.raw_log("");
        self.ticks += 10;
        self.klog("BOOT_COMPLETE");
    }
    pub fn next_boot_line(&mut self) -> Option<String> {
        if self.log.is_empty() {
            self.generate_boot_log();
        }
        if self.boot_index < self.log.len() {
            let line = self.log[self.boot_index].clone();
            self.boot_index += 1;
            Some(line)
        } else {
            None
        }
    }
    pub fn tick(&mut self) {
        self.ticks += 1;
    }
    pub fn uptime_ms(&self) -> u64 {
        self.ticks / 1000
    }

    /// Initialize kernel with persistence loading
    pub async fn init(&mut self) {
        self.fs.load_from_persistence().await;
    }

    /// Save kernel state to persistence
    pub async fn save(&self) {
        self.fs.save_to_persistence().await;
    }
}
