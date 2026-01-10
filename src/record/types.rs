//! RECORD file types and hashing for Python wheels

use std::io::Read;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::Digest;
use sha2::Sha256;

use crate::error::RecordError;

/// Single entry in RECORD file
#[derive(Debug, Clone)]
pub struct RecordEntry {
    pub path: String,
    pub hash: Option<String>,
    pub size: Option<u64>,
}

impl RecordEntry {
    pub fn new(path: String, hash: Option<String>, size: Option<u64>) -> Self {
        Self { path, hash, size }
    }
}

/// Complete RECORD file
#[derive(Debug, Clone, Default)]
pub struct Record {
    pub entries: Vec<RecordEntry>,
}

impl Record {
    /// Parse RECORD from CSV content
    pub fn parse(content: &str) -> Result<Self, RecordError> {
        let mut entries = Vec::new();
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(content.as_bytes());

        for result in reader.records() {
            let record = result.map_err(|e| RecordError::InvalidCsv(e.to_string()))?;

            let path = record.get(0).unwrap_or("").to_string();
            if path.is_empty() {
                continue;
            }

            let hash = record
                .get(1)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            let size = record
                .get(2)
                .filter(|s| !s.is_empty())
                .and_then(|s| s.parse().ok());

            entries.push(RecordEntry { path, hash, size });
        }

        Ok(Record { entries })
    }

    /// Serialize RECORD to CSV format
    pub fn serialize(&self) -> String {
        let mut writer = csv::Writer::from_writer(Vec::new());

        for entry in &self.entries {
            writer
                .write_record([
                    &entry.path,
                    entry.hash.as_deref().unwrap_or(""),
                    &entry.size.map(|s| s.to_string()).unwrap_or_default(),
                ])
                .unwrap();
        }

        String::from_utf8(writer.into_inner().unwrap()).unwrap()
    }

    /// Find entry by path
    pub fn find(&self, path: &str) -> Option<&RecordEntry> {
        self.entries.iter().find(|e| e.path == path)
    }

    /// Find entry by path (mutable)
    pub fn find_mut(&mut self, path: &str) -> Option<&mut RecordEntry> {
        self.entries.iter_mut().find(|e| e.path == path)
    }
}

/// Compute SHA256 hash in wheel format: sha256=<base64url_no_padding>
pub fn hash_content(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let digest = hasher.finalize();
    let encoded = URL_SAFE_NO_PAD.encode(&digest);
    format!("sha256={}", encoded)
}

/// Compute SHA256 hash of a reader's contents
pub fn hash_reader<R: Read>(mut reader: R) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let digest = hasher.finalize();
    let encoded = URL_SAFE_NO_PAD.encode(&digest);
    Ok(format!("sha256={}", encoded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_content() {
        let content = b"Hello, World!";
        let hash = hash_content(content);
        // Hash format is "sha256=<base64url_no_padding>"
        assert!(hash.starts_with("sha256="));
        // The base64 part shouldn't contain padding '=' (URL-safe no padding)
        let base64_part = hash.strip_prefix("sha256=").unwrap();
        assert!(!base64_part.contains('='), "Base64 should not have padding");
    }

    #[test]
    fn test_record_parse() {
        let content = r#"test_package/__init__.py,sha256=abc123,100
test_package-1.0.0.dist-info/METADATA,sha256=def456,200
test_package-1.0.0.dist-info/RECORD,,"#;

        let record = Record::parse(content).unwrap();
        assert_eq!(record.entries.len(), 3);
        assert_eq!(record.entries[0].path, "test_package/__init__.py");
        assert_eq!(record.entries[0].hash, Some("sha256=abc123".to_string()));
        assert_eq!(record.entries[0].size, Some(100));
        assert!(record.entries[2].hash.is_none());
    }

    #[test]
    fn test_record_roundtrip() {
        let original = r#"test/__init__.py,sha256=abc,10
test/RECORD,,"#;

        let record = Record::parse(original).unwrap();
        let serialized = record.serialize();
        let reparsed = Record::parse(&serialized).unwrap();

        assert_eq!(record.entries.len(), reparsed.entries.len());
        assert_eq!(record.entries[0].path, reparsed.entries[0].path);
    }
}
