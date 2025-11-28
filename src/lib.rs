pub mod grub;
pub mod kernel;
pub mod memory;
pub mod neofetch;
pub mod process;
pub mod python;
pub mod shell;
pub mod system;
pub mod vfs;

pub use grub::{GrubMenu, Memtest};
pub use system::System;
