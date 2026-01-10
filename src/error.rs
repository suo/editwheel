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
