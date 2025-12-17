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

        vec![
            "".to_string(),
            "GRUB loading.".to_string(),
            "".to_string(),
            "Welcome to GRUB!".to_string(),
            "".to_string(),
            "Loading Linux ".to_string() + &kernel.version + " ...",
            "Loading initial ramdisk ...".to_string(),
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
