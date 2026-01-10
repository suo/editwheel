//! Error types for editwheel-rs

use thiserror::Error;

/// Main error type for wheel operations
#[derive(Error, Debug)]
pub enum WheelError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Invalid wheel: {0}")]
    InvalidWheel(String),

    #[error("Metadata error: {0}")]
    Metadata(#[from] MetadataError),

    #[error("Record error: {0}")]
    Record(#[from] RecordError),

    #[error("ELF error: {0}")]
    Elf(#[from] ElfError),

    #[error("WHEEL info error: {0}")]
    WheelInfo(#[from] WheelInfoError),

    #[error("Glob pattern error: {0}")]
    GlobPattern(#[from] glob::PatternError),
}

/// Errors related to METADATA parsing
#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

/// Errors related to RECORD file
#[derive(Error, Debug)]
pub enum RecordError {
    #[error("Invalid CSV: {0}")]
    InvalidCsv(String),

    #[error("Hash mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}

/// Errors related to ELF file operations
#[derive(Error, Debug)]
pub enum ElfError {
    #[error("Invalid ELF file: {0}")]
    InvalidElf(String),

    #[error("Unsupported architecture: {0}")]
    UnsupportedArchitecture(String),

    #[error("File not found in wheel: {0}")]
    FileNotFound(String),

    #[error("LIEF error: {0}")]
    Lief(String),
}

/// Errors related to WHEEL file parsing
#[derive(Error, Debug)]
pub enum WheelInfoError {
    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid tag format: {0}")]
    InvalidTag(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

/// Result of validating a wheel
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Individual validation error
#[derive(Debug)]
pub enum ValidationError {
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    MissingFile {
        path: String,
    },
    ExtraFile {
        path: String,
    },
}
