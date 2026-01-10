//! Types for ELF file information and modifications

/// Information extracted from an ELF file
#[derive(Debug, Clone, Default)]
pub struct ElfInfo {
    /// RPATH (legacy, DT_RPATH)
    pub rpath: Option<String>,
    /// RUNPATH (preferred, DT_RUNPATH)
    pub runpath: Option<String>,
    /// List of needed libraries (DT_NEEDED)
    pub needed: Vec<String>,
    /// SONAME of the library
    pub soname: Option<String>,
}

/// Represents a modification to be applied to an ELF file
#[derive(Debug, Clone)]
pub enum ElfModification {
    /// Set the RPATH (DT_RPATH)
    SetRpath(String),
    /// Set the RUNPATH (DT_RUNPATH) - preferred over RPATH
    SetRunpath(String),
}
