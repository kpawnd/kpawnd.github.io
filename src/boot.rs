use crate::memory;
use std::collections::HashMap;

/// Bootloader types
#[derive(Clone, Debug)]
pub enum BootloaderType {
    Grub,
    SystemdBoot,
    Lilo,
    Syslinux,
}

/// Kernel simulation configuration
#[derive(Clone, Debug)]
pub struct KernelConfig {
    pub version: String,
    pub modules: Vec<String>,
    pub initrd: Option<String>,
    pub cmdline: String,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            version: "6.1.0-kpawnd".to_string(),
            modules: vec![
                "ext4".to_string(),
                "ahci".to_string(),
                "xhci_hcd".to_string(),
                "ehci_hcd".to_string(),
            ],
            initrd: Some("initrd.img-6.1.0-kpawnd".to_string()),
            cmdline: "root=/dev/sda1 ro quiet".to_string(),
        }
    }
}

/// Bootloader trait for different implementations
pub trait Bootloader {
    fn name(&self) -> &str;
    fn simulate_boot(&self, kernel: &KernelConfig, memory: &mut memory::Memory) -> Vec<String>;
}

/// GRUB bootloader implementation
pub struct GrubBootloader;

impl Bootloader for GrubBootloader {
    fn name(&self) -> &str {
        "GRUB"
    }

    fn simulate_boot(&self, kernel: &KernelConfig, memory: &mut memory::Memory) -> Vec<String> {
        // GRUB allocates memory for itself and kernel loading
        let grub_size = 512 * 1024; // 512KB for GRUB
        let _ = memory.alloc(grub_size);

        let total_mib = (memory.total as f64) / (1024.0 * 1024.0);
        let avail_mib = (memory.free as f64) / (1024.0 * 1024.0);
        let initrd = kernel
            .initrd
            .clone()
            .unwrap_or_else(|| "initrd.img".to_string());
        let quiet_mode = kernel
            .cmdline
            .split_whitespace()
            .any(|token| token == "quiet");

        if quiet_mode {
            return vec![
                "".to_string(),
                "GRUB loading.".to_string(),
                "".to_string(),
                format!("Loading Linux {} ...", kernel.version),
                format!("Loading initial ramdisk {} ...", initrd),
                "".to_string(),
                format!("[    0.000000] Linux version {}", kernel.version),
                "[    0.027114] kernel: command line parameters loaded".to_string(),
                "[    0.041903] kernel: mounting root filesystem".to_string(),
                "[    0.054228] EXT4-fs (sda1): mounted filesystem with ordered data mode"
                    .to_string(),
                "[    0.066441] systemd[1]: systemd 255 running in system mode".to_string(),
                "[    0.074229] systemd[1]: Detected architecture x86-64.".to_string(),
                "[    0.082511] systemd[1]: Starting Journal Service...".to_string(),
                "[    0.089342] systemd[1]: Started Journal Service.".to_string(),
                "[    0.095870] systemd[1]: Starting Load Kernel Modules...".to_string(),
                "[    0.102619] systemd[1]: Started udev Coldplug all Devices.".to_string(),
                "[    0.109441] systemd[1]: Reached target Local File Systems.".to_string(),
                "[    0.115228] systemd[1]: Starting Network Manager...".to_string(),
                "[    0.121771] systemd[1]: Started Network Manager.".to_string(),
                "[    0.128444] systemd[1]: Reached target Network.".to_string(),
                "[    0.136981] systemd[1]: Started Getty on tty1.".to_string(),
                "[    0.146205] systemd[1]: Reached target Multi-User System.".to_string(),
                "".to_string(),
            ];
        }

        vec![
            "".to_string(),
            "GRUB loading.".to_string(),
            "".to_string(),
            format!("Loading Linux {} ...", kernel.version),
            format!("Loading initial ramdisk {} ...", initrd),
            "".to_string(),
            format!(
                "[    0.000000] Linux version {} (gcc version 12.2.0) #1 SMP PREEMPT_DYNAMIC",
                kernel.version
            ),
            format!("[    0.000000] Command line: {}", kernel.cmdline),
            "[    0.000000] x86/fpu: x87 FPU will use FXSAVE".to_string(),
            "[    0.000000] BIOS-provided physical RAM map:".to_string(),
            "[    0.000000] BIOS-e820: [mem 0x0000000000000000-0x0000000001ffffff] usable"
                .to_string(),
            "[    0.000000] NX (Execute Disable) protection: active".to_string(),
            "[    0.000000] DMI: kpawnd WASM VM/Virtual Board, BIOS 2.06 04/08/2026".to_string(),
            format!(
                "[    0.000000] Memory: {:.1}MiB/{:.1}MiB available",
                avail_mib, total_mib
            ),
            "[    0.021337] ACPI: Interpreter enabled".to_string(),
            "[    0.044200] PCI: Using configuration type 1 for base access".to_string(),
            "[    0.073910] ahci 0000:00:1f.2: AHCI 0001.0301 32 slots 1 ports".to_string(),
            "[    0.086542] usb usb1: New USB device found, idVendor=1d6b, idProduct=0002"
                .to_string(),
            "[    0.098773] EXT4-fs (sda1): mounted filesystem with ordered data mode".to_string(),
            "[    0.109041] VFS: Mounted root (ext4 filesystem) readonly on device 8:1.".to_string(),
            "[    0.120012] systemd[1]: systemd 255 running in system mode (+PAM +AUDIT +SELINUX)"
                .to_string(),
            "[    0.129884] systemd[1]: Detected architecture x86-64.".to_string(),
            "[    0.135611] systemd[1]: Listening on Journal Socket.".to_string(),
            "[    0.142007] systemd[1]: Listening on Journal Socket (/dev/log).".to_string(),
            "[    0.149965] systemd[1]: Started Journal Service.".to_string(),
            "[    0.156732] systemd-journald[221]: Received client request to flush runtime journal."
                .to_string(),
            "[    0.164019] systemd[1]: Started udev Coldplug all Devices.".to_string(),
            "[    0.170882] systemd[1]: Reached target Local File Systems.".to_string(),
            "[    0.177331] systemd[1]: Started Network Manager.".to_string(),
            "[    0.184551] systemd[1]: Reached target Network.".to_string(),
            "[    0.191004] systemd[1]: Started Getty on tty1.".to_string(),
            "[    0.199900] systemd[1]: Reached target Multi-User System.".to_string(),
            "".to_string(),
        ]
    }
}

/// systemd-boot implementation
pub struct SystemdBootloader;

impl Bootloader for SystemdBootloader {
    fn name(&self) -> &str {
        "systemd-boot"
    }

    fn simulate_boot(&self, kernel: &KernelConfig, memory: &mut memory::Memory) -> Vec<String> {
        // systemd-boot allocates memory for itself
        let boot_size = 256 * 1024; // 256KB for systemd-boot
        let _ = memory.alloc(boot_size);

        vec![
            "".to_string(),
            "systemd-boot ".to_string() + &kernel.version,
            "".to_string(),
            "Loading Linux ".to_string() + &kernel.version + " ...",
            "Loading initial ramdisk ...".to_string(),
            format!("Command line: {}", kernel.cmdline),
            "".to_string(),
            "Starting kernel ...".to_string(),
            "".to_string(),
            // Kernel messages start here
            format!(
                "Linux version {} (wasm-pack) #1 SMP PREEMPT_DYNAMIC",
                kernel.version
            ),
            format!("Command line: {}", kernel.cmdline),
            "".to_string(),
            "x86_64 CPU features: SSE SSE2 SSE3 SSSE3 SSE4.1 SSE4.2 AVX AVX2".to_string(),
            format!(
                "Memory: {:.1}MB total, {:.1}MB available",
                (memory.total as f64) / (1024.0 * 1024.0),
                (memory.free as f64) / (1024.0 * 1024.0)
            ),
            "Kernel command line: ".to_string() + &kernel.cmdline,
            "".to_string(),
            "Loading kernel modules...".to_string(),
            "[ OK ] Loading kernel module: ext4".to_string(),
            "[ OK ] Loading kernel module: ahci".to_string(),
            "[ OK ] Loading kernel module: xhci_hcd".to_string(),
            "[ OK ] Loading kernel module: ehci_hcd".to_string(),
            "".to_string(),
            "[ OK ] Kernel initialized successfully.".to_string(),
            "[ OK ] Starting init process...".to_string(),
            "".to_string(),
        ]
    }
}

/// Boot manager that handles different bootloaders
pub struct BootManager {
    bootloaders: HashMap<String, Box<dyn Bootloader>>,
    current_bootloader: String,
    kernel_config: KernelConfig,
}

impl BootManager {
    pub fn new() -> Self {
        let mut bootloaders = HashMap::new();
        bootloaders.insert(
            "grub".to_string(),
            Box::new(GrubBootloader) as Box<dyn Bootloader>,
        );
        bootloaders.insert(
            "systemd-boot".to_string(),
            Box::new(SystemdBootloader) as Box<dyn Bootloader>,
        );

        Self {
            bootloaders,
            current_bootloader: "grub".to_string(),
            kernel_config: KernelConfig::default(),
        }
    }

    pub fn set_bootloader(&mut self, name: &str) -> Result<(), String> {
        if self.bootloaders.contains_key(name) {
            self.current_bootloader = name.to_string();
            Ok(())
        } else {
            Err(format!("Bootloader '{}' not found", name))
        }
    }

    pub fn list_bootloaders(&self) -> Vec<String> {
        self.bootloaders.keys().cloned().collect()
    }

    pub fn get_current_bootloader(&self) -> &str {
        &self.current_bootloader
    }

    pub fn update_kernel_config(&mut self, config: KernelConfig) {
        self.kernel_config = config;
    }

    pub fn get_kernel_config(&self) -> &KernelConfig {
        &self.kernel_config
    }

    pub fn set_cmdline(&mut self, cmdline: &str) {
        self.kernel_config.cmdline = cmdline.to_string();
    }

    pub fn get_cmdline(&self) -> String {
        self.kernel_config.cmdline.clone()
    }

    pub fn set_kernel_version(&mut self, version: &str) {
        self.kernel_config.version = version.to_string();
        self.kernel_config.initrd = Some(format!("initrd.img-{}", version));
    }

    pub fn get_kernel_version(&self) -> String {
        self.kernel_config.version.clone()
    }

    pub fn simulate_boot_sequence(&self, memory: &mut memory::Memory) -> Vec<String> {
        if let Some(bootloader) = self.bootloaders.get(&self.current_bootloader) {
            bootloader.simulate_boot(&self.kernel_config, memory)
        } else {
            vec!["Error: No bootloader configured".to_string()]
        }
    }

    /// Add a custom bootloader (for extensibility)
    pub fn add_bootloader(&mut self, name: String, bootloader: Box<dyn Bootloader>) {
        self.bootloaders.insert(name, bootloader);
    }
}

impl Default for BootManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel simulation (basic)
pub struct KernelSimulator {
    config: KernelConfig,
    loaded_modules: Vec<String>,
    running: bool,
}

impl KernelSimulator {
    pub fn new(config: KernelConfig) -> Self {
        Self {
            config,
            loaded_modules: vec![],
            running: false,
        }
    }

    pub fn start(&mut self) -> Vec<String> {
        self.running = true;
        let mut output = vec![
            format!(
                "Linux version {} (wasm-pack) #1 SMP PREEMPT_DYNAMIC",
                self.config.version
            ),
            "Command line: ".to_string() + &self.config.cmdline,
            "".to_string(),
            "Kernel command line: ".to_string() + &self.config.cmdline,
            "".to_string(),
        ];

        // Load modules
        for module in &self.config.modules {
            self.loaded_modules.push(module.clone());
            output.push(format!("Loading kernel module: {}", module));
        }

        output.extend(vec![
            "".to_string(),
            "Kernel initialized successfully.".to_string(),
            "Starting init process...".to_string(),
            "".to_string(),
        ]);

        output
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn get_loaded_modules(&self) -> &[String] {
        &self.loaded_modules
    }
}
