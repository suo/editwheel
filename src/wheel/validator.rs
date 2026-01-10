//! Wheel validation - verify all hashes in RECORD match actual contents

use std::collections::HashSet;
use std::io::Read;
use std::io::Seek;

use zip::ZipArchive;

use crate::error::ValidationError;
use crate::error::ValidationResult;
use crate::error::WheelError;
use crate::record::Record;
use crate::record::hash_content;

/// Validate all file hashes in a wheel against the RECORD file
pub fn validate_wheel<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    record: &Record,
) -> Result<ValidationResult, WheelError> {
    let mut result = ValidationResult::default();

    // Build set of files in archive
    let mut archive_files: HashSet<String> = HashSet::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        // Skip directories
        if !file.name().ends_with('/') {
            archive_files.insert(file.name().to_string());
        }
    }

    // Check each RECORD entry
    for entry in &record.entries {
        // Skip RECORD itself (it has no hash)
        if entry.hash.is_none() {
            continue;
        }

        // Check if file exists in archive
        if !archive_files.contains(&entry.path) {
            result.errors.push(ValidationError::MissingFile {
                path: entry.path.clone(),
            });
            continue;
        }

        // Read file contents and compute hash
        let mut file = archive.by_name(&entry.path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let actual_hash = hash_content(&contents);
        let expected_hash = entry.hash.as_ref().unwrap();

        if &actual_hash != expected_hash {
            result.errors.push(ValidationError::HashMismatch {
                path: entry.path.clone(),
                expected: expected_hash.clone(),
                actual: actual_hash,
            });
        }

        // Remove from archive_files set to track what's been checked
        archive_files.remove(&entry.path);
    }

    // Check for files in archive but not in RECORD
    // (excluding RECORD itself which is allowed to not have a hash entry for itself)
    for path in archive_files {
        if !path.ends_with("/RECORD") {
            result.errors.push(ValidationError::ExtraFile { path });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::io::Write;

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;
    use crate::record::RecordEntry;

    fn create_valid_wheel() -> (Vec<u8>, Record) {
        let mut buf = Cursor::new(Vec::new());
        let content = b"test content";
        let content_hash = hash_content(content);

        {
            let mut zip = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();

            zip.start_file("test.py", options).unwrap();
            zip.write_all(content).unwrap();

            zip.start_file("pkg-1.0.dist-info/RECORD", options).unwrap();
            zip.write_all(b"").unwrap();

            zip.finish().unwrap();
        }

        let record = Record {
            entries: vec![
                RecordEntry::new(
                    "test.py".to_string(),
                    Some(content_hash),
                    Some(content.len() as u64),
                ),
                RecordEntry::new("pkg-1.0.dist-info/RECORD".to_string(), None, None),
            ],
        };

        (buf.into_inner(), record)
    }

    #[test]
    fn test_validate_valid_wheel() {
        let (wheel_data, record) = create_valid_wheel();
        let mut archive = ZipArchive::new(Cursor::new(wheel_data)).unwrap();

        let result = validate_wheel(&mut archive, &record).unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_hash_mismatch() {
        let (wheel_data, mut record) = create_valid_wheel();
        // Corrupt the expected hash
        record.entries[0].hash = Some("sha256=wronghash".to_string());

        let mut archive = ZipArchive::new(Cursor::new(wheel_data)).unwrap();
        let result = validate_wheel(&mut archive, &record).unwrap();

        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        matches!(&result.errors[0], ValidationError::HashMismatch { .. });
    }
}
