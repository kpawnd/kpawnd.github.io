use std::hash::{DefaultHasher, Hash, Hasher};
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
    advanced_mode: bool,
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
                "Memory test (memtest86+)".to_string(),
            ],
            edit_mode: false,
            cmdline_mode: false,
            cmdline_buffer: String::new(),
            edit_buffer: Vec::new(),
            advanced_mode: false,
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
        if self.edit_mode || self.cmdline_mode || self.advanced_mode {
            return true; // Don't countdown in special modes or advanced mode
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
        self.timer == 0 && !self.edit_mode && !self.cmdline_mode && !self.advanced_mode
    }

    #[wasm_bindgen]
    pub fn enter_advanced_mode(&mut self) {
        self.advanced_mode = true;
        self.selected = 0;
        self.timer = 5; // Reset timer
        self.entries = vec![
            "Back to main menu".to_string(),
            "kpawnd GNU/Linux, with Linux 6.1.0-kpawnd".to_string(),
            "kpawnd GNU/Linux, with Linux 6.1.0-kpawnd (recovery mode)".to_string(),
            "Memory test (memtest86+)".to_string(),
        ];
    }

    #[wasm_bindgen]
    pub fn exit_advanced_mode(&mut self) {
        self.advanced_mode = false;
        self.selected = 0;
        self.timer = 5;
        self.entries = vec![
            "kpawnd GNU/Linux".to_string(),
            "Advanced options for kpawnd GNU/Linux".to_string(),
            "Memory test (memtest86+)".to_string(),
        ];
    }

    #[wasm_bindgen]
    pub fn is_advanced_mode(&self) -> bool {
        self.advanced_mode
    }
}

#[wasm_bindgen]
pub struct Memtest {
    tests: Vec<String>,
    current_test: usize,
    progress: u32,
    total_mem: u32,
    test_memory: Vec<u8>,
    errors: u32,
}

#[wasm_bindgen]
impl Memtest {
    #[wasm_bindgen(constructor)]
    pub fn new(mem_size: u32) -> Self {
        // Allocate a reasonable test memory size (limit to 16MB for browser)
        let test_size = (mem_size * 1024 * 1024).min(16 * 1024 * 1024) as usize;
        let test_memory = vec![0u8; test_size];

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
            test_memory,
            errors: 0,
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
        // Perform actual memory testing based on current test
        match self.current_test {
            0 => self.test_address_own_address(),
            1 => self.test_moving_inversions_ones_zeros(),
            2 => self.test_moving_inversions_8bit(),
            3 => self.test_moving_inversions_random(),
            4 => self.test_block_move(),
            5 => self.test_moving_inversions_32bit(),
            6 => self.test_random_sequence(),
            7 => self.test_modulo_20(),
            8 => self.test_bit_fade(),
            _ => {}
        }

        self.progress += 10;
        if self.progress >= 100 {
            self.progress = 0;
            self.current_test += 1;
        }
        self.current_test < self.tests.len()
    }

    #[wasm_bindgen]
    pub fn get_current_line(&self) -> String {
        if self.current_test >= self.tests.len() {
            return format!(
                "\n** Pass complete, {} errors, press Esc to exit **",
                self.errors
            );
        }

        let test_name = &self.tests[self.current_test];
        let bar_length = 20;
        let filled = (self.progress / 5) as usize;
        let empty = bar_length - filled;
        let progress_bar = format!("{}{}", "=".repeat(filled), " ".repeat(empty));

        format!(
            "Test {}: {} [{}] {}% (Errors: {})",
            self.current_test + 1,
            test_name,
            progress_bar,
            self.progress,
            self.errors
        )
    }

    #[wasm_bindgen]
    pub fn is_complete(&self) -> bool {
        self.current_test >= self.tests.len()
    }

    #[wasm_bindgen]
    pub fn get_errors(&self) -> u32 {
        self.errors
    }
}

// Memory testing implementations
impl Memtest {
    fn test_address_own_address(&mut self) {
        let chunk_size = 4096; // Test in 4KB chunks
        let chunks = self.test_memory.len() / chunk_size;

        for chunk in 0..chunks {
            let start = chunk * chunk_size;
            let end = start + chunk_size;

            // Write address pattern
            for i in start..end {
                let addr = (i % 256) as u8;
                self.test_memory[i] = addr;
            }

            // Read back and verify
            for i in start..end {
                let expected = (i % 256) as u8;
                if self.test_memory[i] != expected {
                    self.errors += 1;
                }
            }
        }
    }

    fn test_moving_inversions_ones_zeros(&mut self) {
        let pattern1 = 0xFFu8; // All ones
        let pattern2 = 0x00u8; // All zeros

        // First pass: write pattern1, then pattern2
        for i in 0..self.test_memory.len() {
            self.test_memory[i] = pattern1;
        }
        for i in 0..self.test_memory.len() {
            if self.test_memory[i] != pattern1 {
                self.errors += 1;
            }
            self.test_memory[i] = pattern2;
        }

        // Second pass: verify pattern2, then pattern1
        for i in 0..self.test_memory.len() {
            if self.test_memory[i] != pattern2 {
                self.errors += 1;
            }
            self.test_memory[i] = pattern1;
        }
        for i in 0..self.test_memory.len() {
            if self.test_memory[i] != pattern1 {
                self.errors += 1;
            }
        }
    }

    fn test_moving_inversions_8bit(&mut self) {
        let patterns = [0xAAu8, 0x55u8]; // Alternating bit patterns

        for &pattern in &patterns {
            // Write pattern
            for i in 0..self.test_memory.len() {
                self.test_memory[i] = pattern;
            }

            // Verify pattern
            for i in 0..self.test_memory.len() {
                if self.test_memory[i] != pattern {
                    self.errors += 1;
                }
            }
        }
    }

    fn test_moving_inversions_random(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Generate pseudo-random pattern based on address
        for i in 0..self.test_memory.len() {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            let pattern = (hasher.finish() % 256) as u8;
            self.test_memory[i] = pattern;
        }

        // Verify pattern
        for i in 0..self.test_memory.len() {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            let expected = (hasher.finish() % 256) as u8;
            if self.test_memory[i] != expected {
                self.errors += 1;
            }
        }
    }

    fn test_block_move(&mut self) {
        let block_size = 1024;
        let mut temp_buffer = vec![0u8; block_size];

        for i in (0..self.test_memory.len()).step_by(block_size) {
            let end = (i + block_size).min(self.test_memory.len());

            // Copy block to temp
            temp_buffer[..(end - i)].copy_from_slice(&self.test_memory[i..end]);

            // Write different pattern
            for j in i..end {
                self.test_memory[j] = 0xFF;
            }

            // Copy back
            self.test_memory[i..end].copy_from_slice(&temp_buffer[..(end - i)]);

            // Verify
            for j in i..end {
                let mut hasher = DefaultHasher::new();
                j.hash(&mut hasher);
                let expected = (hasher.finish() % 256) as u8;
                if self.test_memory[j] != expected {
                    self.errors += 1;
                }
            }
        }
    }

    fn test_moving_inversions_32bit(&mut self) {
        let patterns = [0xFFFFFFFFu32, 0x00000000u32];

        for &pattern_u32 in &patterns {
            let pattern_bytes = pattern_u32.to_le_bytes();

            for i in (0..self.test_memory.len()).step_by(4) {
                if i + 4 <= self.test_memory.len() {
                    self.test_memory[i..i + 4].copy_from_slice(&pattern_bytes);
                }
            }

            // Verify
            for i in (0..self.test_memory.len()).step_by(4) {
                if i + 4 <= self.test_memory.len() && self.test_memory[i..i + 4] != pattern_bytes {
                    self.errors += 1;
                }
            }
        }
    }

    fn test_random_sequence(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Fill with pseudo-random sequence
        for i in 0..self.test_memory.len() {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            self.test_memory[i] = (hasher.finish() % 256) as u8;
        }

        // Verify sequence
        for i in 0..self.test_memory.len() {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            let expected = (hasher.finish() % 256) as u8;
            if self.test_memory[i] != expected {
                self.errors += 1;
            }
        }
    }

    fn test_modulo_20(&mut self) {
        let patterns = [0xFFu8, 0x00u8];

        for &pattern in &patterns {
            for i in 0..self.test_memory.len() {
                if i % 20 == 0 {
                    self.test_memory[i] = pattern;
                }
            }

            // Verify
            for i in 0..self.test_memory.len() {
                if i % 20 == 0 && self.test_memory[i] != pattern {
                    self.errors += 1;
                }
            }
        }
    }

    fn test_bit_fade(&mut self) {
        // Simplified bit fade test - just write and read back
        let pattern1 = 0xAAu8;
        let pattern2 = 0x55u8;

        // Write pattern1
        for i in 0..self.test_memory.len() {
            self.test_memory[i] = pattern1;
        }

        // "Wait" simulation - just verify immediately
        for i in 0..self.test_memory.len() {
            if self.test_memory[i] != pattern1 {
                self.errors += 1;
            }
        }

        // Write pattern2
        for i in 0..self.test_memory.len() {
            self.test_memory[i] = pattern2;
        }

        // Verify pattern2
        for i in 0..self.test_memory.len() {
            if self.test_memory[i] != pattern2 {
                self.errors += 1;
            }
        }
    }
}
