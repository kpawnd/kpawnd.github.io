use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GrubMenu {
    selected: usize,
    timer: u32,
    entries: Vec<String>,
    edit_mode: bool,
    cmdline_mode: bool,
    cmdline_buffer: String,
    edit_buffer: Vec<String>,
}

impl Default for GrubMenu {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl GrubMenu {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        GrubMenu {
            selected: 0,
            timer: 5,
            entries: vec![
                "kpawnd GNU/Linux".to_string(),
                "Advanced options for kpawnd GNU/Linux".to_string(),
                "UEFI Firmware Settings".to_string(),
            ],
            edit_mode: false,
            cmdline_mode: false,
            cmdline_buffer: String::new(),
            edit_buffer: Vec::new(),
        }
    }

    #[wasm_bindgen]
    pub fn render(&self) -> String {
        if self.cmdline_mode {
            return self.render_cmdline();
        }
        if self.edit_mode {
            return self.render_edit();
        }

        let mut output = String::new();

        // Authentic GRUB 2.06 look
        output.push('\n');
        output.push('\n');
        output.push('\n');
        output.push_str("                            GNU GRUB  version 2.06\n");
        output.push('\n');
        output.push_str(
            " ┌────────────────────────────────────────────────────────────────────────────┐\n",
        );

        for (i, entry) in self.entries.iter().enumerate() {
            if i == self.selected {
                // White background, black text for selected
                output.push_str(&format!(" │\x1b[HIGHLIGHT]*{:<75}\x1b[NORMAL]│\n", entry));
            } else {
                output.push_str(&format!(" │ {:<76}│\n", entry));
            }
        }

        // Fill remaining space to make box consistent
        for _ in 0..(12 - self.entries.len().min(12)) {
            output.push_str(
                " │                                                                            │\n",
            );
        }

        output.push_str(
            " │                                                                            │\n",
        );
        output.push_str(
            " │                                                                            │\n",
        );
        output.push_str(
            " │                                                                            │\n",
        );
        output.push_str(
            " └────────────────────────────────────────────────────────────────────────────┘\n",
        );
        output.push('\n');
        output.push_str("      Use the ▲ and ▼ keys to select which entry is highlighted.\n");
        output.push_str("      Press enter to boot the selected OS, `e' to edit the commands\n");
        output.push_str("      before booting or `c' for a command-line.\n");
        output.push('\n');
        output.push_str(&format!(
            "   The highlighted entry will be executed automatically in {}s.    ",
            self.timer
        ));

        output
    }

    fn render_edit(&self) -> String {
        let mut output = String::new();
        output.push('\n');
        output.push('\n');
        output.push('\n');
        output.push_str("                            GNU GRUB  version 2.06\n");
        output.push('\n');
        output.push_str(
            " ┌────────────────────────────────────────────────────────────────────────────┐\n",
        );

        for line in &self.edit_buffer {
            let display = if line.len() > 76 { &line[..76] } else { line };
            output.push_str(&format!(" │{:<77}│\n", display));
        }

        // Fill remaining
        for _ in 0..(15 - self.edit_buffer.len().min(15)) {
            output.push_str(
                " │                                                                            │\n",
            );
        }

        output.push_str(
            " └────────────────────────────────────────────────────────────────────────────┘\n",
        );
        output.push('\n');
        output.push_str("      Minimum Emacs-like screen editing is supported. TAB lists\n");
        output.push_str("      completions. Press Ctrl-x or F10 to boot, Ctrl-c or F2 for\n");
        output.push_str(
            "      a command-line or ESC to discard edits and return to the GRUB menu.\n",
        );

        output
    }

    fn render_cmdline(&self) -> String {
        let mut output = String::new();
        output.push('\n');
        output.push('\n');
        output.push('\n');
        output.push_str("                            GNU GRUB  version 2.06\n");
        output.push('\n');
        output
            .push_str("   Minimal BASH-like line editing is supported. For the first word, TAB\n");
        output
            .push_str("   lists possible command completions. Anywhere else TAB lists possible\n");
        output.push_str("   device or file completions.\n");
        output.push('\n');
        output.push_str(&format!("grub> {}\n", self.cmdline_buffer));
        output.push('\n');

        output
    }

    #[wasm_bindgen]
    pub fn enter_edit_mode(&mut self) {
        self.edit_mode = true;
        self.timer = 0;
        self.edit_buffer = vec![
            "setparams 'kpawnd GNU/Linux'".to_string(),
            "".to_string(),
            "    insmod gzio".to_string(),
            "    insmod part_gpt".to_string(),
            "    insmod ext2".to_string(),
            "    search --no-floppy --fs-uuid --set=root wasm-uuid".to_string(),
            "    echo    'Loading Linux 6.1.0-kpawnd ...'".to_string(),
            "    linux   /boot/vmlinuz-6.1.0-kpawnd root=/dev/wasm0 ro quiet".to_string(),
            "    echo    'Loading initial ramdisk ...'".to_string(),
            "    initrd  /boot/initrd.img-6.1.0-kpawnd".to_string(),
        ];
    }

    #[wasm_bindgen]
    pub fn enter_cmdline_mode(&mut self) {
        self.cmdline_mode = true;
        self.timer = 0;
        self.cmdline_buffer = String::new();
    }

    #[wasm_bindgen]
    pub fn exit_special_mode(&mut self) {
        self.edit_mode = false;
        self.cmdline_mode = false;
        self.timer = 5;
    }

    #[wasm_bindgen]
    pub fn is_edit_mode(&self) -> bool {
        self.edit_mode
    }

    #[wasm_bindgen]
    pub fn is_cmdline_mode(&self) -> bool {
        self.cmdline_mode
    }

    #[wasm_bindgen]
    pub fn move_up(&mut self) {
        if !self.edit_mode && !self.cmdline_mode && self.selected > 0 {
            self.selected -= 1;
        }
    }

    #[wasm_bindgen]
    pub fn move_down(&mut self) {
        if !self.edit_mode && !self.cmdline_mode && self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    #[wasm_bindgen]
    pub fn tick(&mut self) -> bool {
        if self.edit_mode || self.cmdline_mode {
            return true; // Don't countdown in special modes
        }
        if self.timer > 0 {
            self.timer -= 1;
            true
        } else {
            false
        }
    }

    #[wasm_bindgen]
    pub fn get_selected(&self) -> usize {
        self.selected
    }

    #[wasm_bindgen]
    pub fn should_boot(&self) -> bool {
        self.timer == 0 && !self.edit_mode && !self.cmdline_mode
    }
}

#[wasm_bindgen]
pub struct Memtest {
    tests: Vec<String>,
    current_test: usize,
    progress: u32,
    total_mem: u32,
}

#[wasm_bindgen]
impl Memtest {
    #[wasm_bindgen(constructor)]
    pub fn new(mem_size: u32) -> Self {
        Memtest {
            tests: vec![
                "Address test, own address".to_string(),
                "Moving inversions, ones & zeros".to_string(),
                "Moving inversions, 8 bit pattern".to_string(),
                "Moving inversions, random pattern".to_string(),
                "Block move, 64 moves".to_string(),
                "Moving inversions, 32 bit pattern".to_string(),
                "Random number sequence".to_string(),
                "Modulo 20, ones & zeros".to_string(),
                "Bit fade test, 90 min, 2 patterns".to_string(),
            ],
            current_test: 0,
            progress: 0,
            total_mem: mem_size,
        }
    }

    #[wasm_bindgen]
    pub fn get_header(&self) -> String {
        format!(
            "Memtest86+ v5.01\n\nTesting {}MB of memory\n",
            self.total_mem
        )
    }

    #[wasm_bindgen]
    pub fn tick(&mut self) -> bool {
        self.progress += 10;
        if self.progress > 100 {
            self.progress = 0;
            self.current_test += 1;
        }
        self.current_test < self.tests.len()
    }

    #[wasm_bindgen]
    pub fn get_current_line(&self) -> String {
        if self.current_test >= self.tests.len() {
            return "\n** Pass complete, no errors, press Esc to exit **".to_string();
        }

        let test_name = &self.tests[self.current_test];
        let bar_length = 20;
        let filled = (self.progress / 5) as usize;
        let empty = bar_length - filled;
        let progress_bar = format!("{}{}", "=".repeat(filled), " ".repeat(empty));

        format!(
            "Test {}: {} [{}] {}%",
            self.current_test + 1,
            test_name,
            progress_bar,
            self.progress
        )
    }

    #[wasm_bindgen]
    pub fn is_complete(&self) -> bool {
        self.current_test >= self.tests.len()
    }
}
