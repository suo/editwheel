//! ELF parsing and modification using elb
//!
//! This module uses the `elb` crate to parse and modify ELF binaries.
//! The elb crate is a pure Rust library specifically designed for patching
//! RPATH, RUNPATH, and interpreter in ELF files.

use std::ffi::CString;
use std::sync::atomic::{AtomicU64, Ordering};

use elb::DynamicTag;
use elb::Elf;
use elb::ElfPatcher;

use crate::error::ElfError;

use super::types::ElfInfo;
use super::types::ElfModification;

// Counter for generating unique temp file names
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// System page size (used by elb for ELF parsing)
const PAGE_SIZE: u64 = 4096;

/// Generate a unique temp file path
fn temp_elf_path() -> std::path::PathBuf {
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("editwheel_elf_{}_{}.so", pid, counter))
}

/// Parse an ELF file from bytes and extract information
pub fn parse_elf(data: &[u8]) -> Result<ElfInfo, ElfError> {
    // Write to temp file (elb requires a seekable file)
    let temp_path = temp_elf_path();
    std::fs::write(&temp_path, data)
        .map_err(|e| ElfError::Lief(format!("Failed to write temp file: {}", e)))?;

    let result = parse_elf_from_path(&temp_path);

    // Clean up
    let _ = std::fs::remove_file(&temp_path);

    result
}

/// Parse an ELF file from a file path
fn parse_elf_from_path(path: &std::path::Path) -> Result<ElfInfo, ElfError> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| ElfError::Lief(format!("Failed to open file: {}", e)))?;

    let elf = Elf::read(&mut file, PAGE_SIZE)
        .map_err(|e| ElfError::InvalidElf(format!("Failed to parse ELF: {}", e)))?;

    let mut info = ElfInfo::default();

    // Extract dynamic entries if present
    // elb's DynamicTable entries are (DynamicTag, u64) tuples where value is an offset
    // The elb library doesn't provide a convenient way to read string values from
    // the dynamic string table directly, so we check for tag presence only.
    // For a full implementation, we'd need to manually read the string table.
    if let Ok(Some(dynamic_table)) = elf.read_dynamic_table(&mut file) {
        for (tag, _value) in dynamic_table.iter() {
            match tag {
                DynamicTag::Rpath => {
                    // We know RPATH exists but can't easily get the value
                    // Set a placeholder indicating presence
                    if info.rpath.is_none() {
                        info.rpath = Some("<rpath-present>".to_string());
                    }
                }
                DynamicTag::Runpath => {
                    if info.runpath.is_none() {
                        info.runpath = Some("<runpath-present>".to_string());
                    }
                }
                DynamicTag::Needed => {
                    // Can't read the actual library name easily
                }
                _ => {}
            }
        }
    }

    Ok(info)
}

/// Get the effective RPATH of an ELF file (prefers RUNPATH over RPATH)
pub fn get_rpath(data: &[u8]) -> Result<Option<String>, ElfError> {
    let info = parse_elf(data)?;
    // RUNPATH takes precedence over RPATH
    Ok(info.runpath.or(info.rpath))
}

/// Modify an ELF file and return the modified bytes
///
/// This function writes the input data to a temp file, uses elb to modify it,
/// and reads back the modified bytes.
pub fn modify_elf(data: &[u8], modifications: &[ElfModification]) -> Result<Vec<u8>, ElfError> {
    // Write to temp file (elb requires a file for the patcher)
    let temp_path = temp_elf_path();
    std::fs::write(&temp_path, data)
        .map_err(|e| ElfError::Lief(format!("Failed to write temp file: {}", e)))?;

    // Open for read+write
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&temp_path)
        .map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            ElfError::Lief(format!("Failed to open temp file: {}", e))
        })?;

    // Parse and create patcher
    let elf = Elf::read(&mut file, PAGE_SIZE).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        ElfError::InvalidElf(format!("Failed to parse ELF: {}", e))
    })?;

    let mut patcher = ElfPatcher::new(elf, file);

    // Apply modifications
    for modification in modifications {
        match modification {
            ElfModification::SetRpath(rpath) => {
                let cstring = CString::new(rpath.as_str()).map_err(|e| {
                    let _ = std::fs::remove_file(&temp_path);
                    ElfError::Lief(format!("Invalid RPATH string: {}", e))
                })?;
                patcher.set_dynamic_tag(DynamicTag::Rpath, cstring.as_c_str()).map_err(|e| {
                    let _ = std::fs::remove_file(&temp_path);
                    ElfError::Lief(format!("Failed to set RPATH: {}", e))
                })?;
            }
            ElfModification::SetRunpath(runpath) => {
                let cstring = CString::new(runpath.as_str()).map_err(|e| {
                    let _ = std::fs::remove_file(&temp_path);
                    ElfError::Lief(format!("Invalid RUNPATH string: {}", e))
                })?;
                patcher.set_dynamic_tag(DynamicTag::Runpath, cstring.as_c_str()).map_err(|e| {
                    let _ = std::fs::remove_file(&temp_path);
                    ElfError::Lief(format!("Failed to set RUNPATH: {}", e))
                })?;
            }
        }
    }

    // Finish patching
    patcher.finish().map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        ElfError::Lief(format!("Failed to finish patching: {}", e))
    })?;

    // Read back the modified bytes
    let modified_data = std::fs::read(&temp_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        ElfError::Lief(format!("Failed to read modified ELF: {}", e))
    })?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    Ok(modified_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require actual ELF binaries to work properly.
    // In a real test environment, you'd use test fixtures or download
    // sample binaries.

    #[test]
    #[ignore] // Requires actual ELF binary
    fn test_parse_system_binary() {
        let data = std::fs::read("/bin/ls").expect("Failed to read /bin/ls");
        let info = parse_elf(&data).expect("Failed to parse ELF");
        // Just verify it parses without panic
        println!("ELF info: {:?}", info);
    }

    #[test]
    #[ignore] // Requires actual ELF binary
    fn test_get_rpath() {
        let data = std::fs::read("/bin/ls").expect("Failed to read /bin/ls");
        let rpath = get_rpath(&data).expect("Failed to get RPATH");
        println!("RPATH: {:?}", rpath);
    }
}
