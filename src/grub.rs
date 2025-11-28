use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GrubMenu {
    selected: usize,
    timer: u32,
    entries: Vec<String>,
}

#[wasm_bindgen]
impl GrubMenu {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        GrubMenu {
            selected: 0,
            timer: 5,
            entries: vec![
                "kpawnd v0.2.0".to_string(),
                "kpawnd v0.2.0 (recovery mode)".to_string(),
                "Memory test (memtest86+)".to_string(),
            ],
        }
    }

    #[wasm_bindgen]
    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str("                         GNU GRUB  version 2.06\n\n");
        output.push_str("   ┌──────────────────────────────────────────────────────────────┐\n");
        
        for (i, entry) in self.entries.iter().enumerate() {
            if i == self.selected {
                output.push_str(&format!("   │ \x1b[HIGHLIGHT]{}\x1b[NORMAL]{} │\n", 
                    entry, 
                    " ".repeat(60 - entry.len())));
            } else {
                output.push_str(&format!("   │  {}{} │\n", 
                    entry, 
                    " ".repeat(59 - entry.len())));
            }
        }
        
        output.push_str("   │                                                              │\n");
        output.push_str("   └──────────────────────────────────────────────────────────────┘\n\n");
        output.push_str("      Use the ↑ and ↓ keys to select which entry is highlighted.\n");
        output.push_str("      Press enter to boot the selected OS, 'e' to edit the\n");
        output.push_str("      commands before booting or 'c' for a command-line.\n\n");
        output.push_str(&format!("   The highlighted entry will be executed automatically in {}s.", self.timer));
        
        output
    }

    #[wasm_bindgen]
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    #[wasm_bindgen]
    pub fn move_down(&mut self) {
        if self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    #[wasm_bindgen]
    pub fn tick(&mut self) -> bool {
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
        self.timer == 0
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
        format!("Memtest86+ v5.01\n\nTesting {}MB of memory\n", self.total_mem)
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
        
        format!("Test {}: {} [{}] {}%", 
            self.current_test + 1, 
            test_name, 
            progress_bar, 
            self.progress)
    }

    #[wasm_bindgen]
    pub fn is_complete(&self) -> bool {
        self.current_test >= self.tests.len()
    }
}
