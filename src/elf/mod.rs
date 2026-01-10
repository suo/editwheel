//! ELF file parsing and modification for editing shared libraries in wheels

mod types;
mod editor;

pub use editor::get_rpath;
pub use editor::modify_elf;
pub use types::ElfInfo;
pub use types::ElfModification;
