use crate::{memory::Memory, process::ProcessTable, vfs::Vfs, process::Scheduler};

pub const VERSION: &str = "0.2.0";
pub const TOTAL_MEM: u32 = 131072; // 128KB

#[derive(PartialEq, Clone, Copy)]
pub enum KernelState { Off, Bios, Boot, Init, Run, Halt }

pub struct Kernel {
    pub state: KernelState,
    pub mem: Memory,
    pub proc: ProcessTable,
    pub fs: Vfs,
    pub ticks: u64,
    log: Vec<String>,
    boot_index: usize,
    pub scheduler: Scheduler,
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
        }
    }
    fn klog(&mut self, subsys: &str, msg: &str) { let ts = self.ticks as f64 * 0.000001; self.log.push(format!("[{:12.6}] {}: {}", ts, subsys, msg)); }
    pub fn generate_boot_log(&mut self) {
        if !self.log.is_empty() { return; }
        self.state = KernelState::Bios; self.ticks = 1; self.klog("bios", "POST");
        self.ticks += 1; self.klog("bios", &format!("mem {}K OK", self.mem.total / 1024));
        self.ticks += 1; self.klog("bios", "boot device: wasm0");
        self.ticks += 10; self.state = KernelState::Boot; self.klog("boot", &format!("kpawnd kernel {}", VERSION));
        self.ticks += 1; self.klog("boot", "loading kernel...");
        self.ticks += 100; self.state = KernelState::Init; self.klog("kernel", "starting");
        self.ticks += 1; let _ = self.mem.alloc(4096); self.klog("mm", &format!("page allocator init, {} pages free", self.mem.free / 4096));
        self.ticks += 1; self.fs.init(); self.klog("vfs", "rootfs mounted");
        self.ticks += 1; self.klog("vfs", "devfs mounted on /dev");
        self.ticks += 1; self.klog("vfs", "procfs mounted on /proc");
        self.ticks += 1; self.klog("tty", "tty0: 80x25");
        self.ticks += 1; let init_pid = self.proc.spawn("init", 0); self.klog("init", &format!("pid {}", init_pid));
        self.ticks += 1; let sh_pid = self.proc.spawn("sh", init_pid); self.scheduler.add(sh_pid); self.klog("init", &format!("spawn sh pid {}", sh_pid));
        self.ticks += 1; self.state = KernelState::Run; self.klog("kernel", "runlevel 3");
        self.ticks += 1; self.klog("kernel", "BOOT_COMPLETE");
    }
    pub fn next_boot_line(&mut self) -> Option<String> { if self.log.is_empty() { self.generate_boot_log(); } if self.boot_index < self.log.len() { let line = self.log[self.boot_index].clone(); self.boot_index += 1; Some(line) } else { None } }
    pub fn tick(&mut self) { self.ticks += 1; }
    pub fn uptime_ms(&self) -> u64 { self.ticks / 1000 }
}
