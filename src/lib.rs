pub mod kernel;
pub mod memory;
pub mod process;
pub mod vfs;
pub mod shell;
pub mod system;
pub mod neofetch;
pub mod python;
pub mod grub;

pub use system::System;
pub use grub::{GrubMenu, Memtest};
