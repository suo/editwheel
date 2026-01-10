//! Wheel writer - writes wheels by copying files
//!
//! This module handles writing modified wheel files using raw copy for maximum
//! performance. Files are copied as raw compressed bytes without decompression.

use std::io::Read;
use std::io::Seek;
use std::io::Write;

use zip::ZipArchive;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use std::collections::HashMap;

use crate::error::WheelError;
use crate::metadata::Metadata;
use crate::record::Record;
use crate::record::RecordEntry;
use crate::record::hash_content;
use crate::wheel_info::WheelInfo;

/// Write a modified wheel by copying files
///
/// # Arguments
/// * `source` - The source wheel archive
/// * `output` - The output writer
/// * `metadata` - The modified metadata to write
/// * `original_record` - The original RECORD for hash preservation
/// * `old_dist_info` - The old dist-info directory name (e.g., "pkg-1.0.0.dist-info")
/// * `new_dist_info` - The new dist-info directory name (e.g., "pkg-1.0.1.dist-info")
pub fn write_modified<R: Read + Seek, W: Write + Seek>(
    source: &mut ZipArchive<R>,
    output: W,
    metadata: &Metadata,
    original_record: &Record,
    old_dist_info: &str,
    new_dist_info: &str,
) -> Result<(), WheelError> {
    let mut writer = ZipWriter::new(output);
    let mut new_record_entries: Vec<RecordEntry> = Vec::new();

    let old_metadata_path = format!("{}/METADATA", old_dist_info);
    let old_record_path = format!("{}/RECORD", old_dist_info);
    let new_metadata_path = format!("{}/METADATA", new_dist_info);
    let new_record_path = format!("{}/RECORD", new_dist_info);

    let needs_rename = old_dist_info != new_dist_info;

    // Phase 1: Copy all files using raw copy (no decompression)
    for i in 0..source.len() {
        let entry = source.by_index_raw(i)?;
        let name = entry.name().to_string();

        // Skip METADATA and RECORD - we'll write new versions
        if name == old_metadata_path || name == old_record_path {
            continue;
        }

        // Determine the new path (handle dist-info rename for version changes)
        let new_name = if needs_rename && name.starts_with(old_dist_info) {
            name.replacen(old_dist_info, new_dist_info, 1)
        } else {
            name.clone()
        };

        // Use raw copy - copies compressed bytes directly without decompression
        if new_name != name {
            writer.raw_copy_file_rename(entry, &new_name)?;
        } else {
            writer.raw_copy_file(entry)?;
        }

        // Preserve original hash from RECORD
        if let Some(record_entry) = original_record.find(&name) {
            new_record_entries.push(RecordEntry::new(
                new_name,
                record_entry.hash.clone(),
                record_entry.size,
            ));
        } else {
            // File not in RECORD - need to compute hash (rare case)
            let mut entry = source.by_index(i)?;
            let mut content = Vec::new();
            std::io::copy(&mut entry, &mut content)?;
            let hash = hash_content(&content);
            new_record_entries.push(RecordEntry::new(
                new_name,
                Some(hash),
                Some(content.len() as u64),
            ));
        }
    }

    // Phase 2: Write new METADATA
    let metadata_bytes = metadata.serialize().into_bytes();
    let metadata_hash = hash_content(&metadata_bytes);
    let metadata_size = metadata_bytes.len() as u64;

    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    writer.start_file(&new_metadata_path, options)?;
    writer.write_all(&metadata_bytes)?;

    new_record_entries.push(RecordEntry::new(
        new_metadata_path,
        Some(metadata_hash),
        Some(metadata_size),
    ));

    // Phase 3: Write new RECORD (RECORD itself has no hash)
    new_record_entries.push(RecordEntry::new(new_record_path.clone(), None, None));

    let record = Record {
        entries: new_record_entries,
    };
    let record_content = record.serialize();

    writer.start_file(&new_record_path, options)?;
    writer.write_all(record_content.as_bytes())?;

    // Finalize the archive
    writer.finish()?;

    Ok(())
}

/// Write a modified wheel with additional modifications (ELF files, WHEEL file)
///
/// This is an extended version of `write_modified` that also handles:
/// - Modified binary files (e.g., .so files with changed RPATH)
/// - Modified WHEEL file (e.g., changed platform tags)
///
/// # Arguments
/// * `source` - The source wheel archive
/// * `output` - The output writer
/// * `metadata` - The modified metadata to write
/// * `original_record` - The original RECORD for hash preservation
/// * `old_dist_info` - The old dist-info directory name
/// * `new_dist_info` - The new dist-info directory name
/// * `modified_files` - Map of file paths to their modified content
/// * `wheel_info` - Optional modified WHEEL info (if None, uses original)
pub fn write_modified_extended<R: Read + Seek, W: Write + Seek>(
    source: &mut ZipArchive<R>,
    output: W,
    metadata: &Metadata,
    original_record: &Record,
    old_dist_info: &str,
    new_dist_info: &str,
    modified_files: &HashMap<String, Vec<u8>>,
    wheel_info: Option<&WheelInfo>,
) -> Result<(), WheelError> {
    let mut writer = ZipWriter::new(output);
    let mut new_record_entries: Vec<RecordEntry> = Vec::new();

    let old_metadata_path = format!("{}/METADATA", old_dist_info);
    let old_record_path = format!("{}/RECORD", old_dist_info);
    let old_wheel_path = format!("{}/WHEEL", old_dist_info);
    let new_metadata_path = format!("{}/METADATA", new_dist_info);
    let new_record_path = format!("{}/RECORD", new_dist_info);
    let new_wheel_path = format!("{}/WHEEL", new_dist_info);

    let needs_rename = old_dist_info != new_dist_info;
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Phase 1: Copy all files, handling modifications
    for i in 0..source.len() {
        let entry = source.by_index_raw(i)?;
        let name = entry.name().to_string();

        // Skip files we'll write new versions of
        if name == old_metadata_path || name == old_record_path {
            continue;
        }

        // Skip WHEEL file if we have a modified version
        if wheel_info.is_some() && name == old_wheel_path {
            continue;
        }

        // Determine the new path (handle dist-info rename for version changes)
        let new_name = if needs_rename && name.starts_with(old_dist_info) {
            name.replacen(old_dist_info, new_dist_info, 1)
        } else {
            name.clone()
        };

        // Check if this file has been modified
        if let Some(modified_content) = modified_files.get(&name) {
            // Write the modified content
            drop(entry); // Release the raw entry
            writer.start_file(&new_name, options)?;
            writer.write_all(modified_content)?;

            // Compute new hash for modified content
            let hash = hash_content(modified_content);
            new_record_entries.push(RecordEntry::new(
                new_name,
                Some(hash),
                Some(modified_content.len() as u64),
            ));
        } else {
            // Preserve original hash from RECORD if available
            if let Some(record_entry) = original_record.find(&name) {
                // Use raw copy - copies compressed bytes directly without decompression
                if new_name != name {
                    writer.raw_copy_file_rename(entry, &new_name)?;
                } else {
                    writer.raw_copy_file(entry)?;
                }

                new_record_entries.push(RecordEntry::new(
                    new_name,
                    record_entry.hash.clone(),
                    record_entry.size,
                ));
            } else {
                // File not in RECORD - need to compute hash (rare case)
                // First drop the raw entry, then read the decompressed content
                drop(entry);
                let mut decompressed = source.by_index(i)?;
                let mut content = Vec::new();
                std::io::copy(&mut decompressed, &mut content)?;
                let hash = hash_content(&content);

                // Write the content normally
                writer.start_file(&new_name, options)?;
                writer.write_all(&content)?;

                new_record_entries.push(RecordEntry::new(
                    new_name,
                    Some(hash),
                    Some(content.len() as u64),
                ));
            }
        }
    }

    // Phase 2: Write new WHEEL file if modified
    if let Some(wheel_info) = wheel_info {
        let wheel_bytes = wheel_info.serialize().into_bytes();
        let wheel_hash = hash_content(&wheel_bytes);
        let wheel_size = wheel_bytes.len() as u64;

        writer.start_file(&new_wheel_path, options)?;
        writer.write_all(&wheel_bytes)?;

        new_record_entries.push(RecordEntry::new(
            new_wheel_path,
            Some(wheel_hash),
            Some(wheel_size),
        ));
    }

    // Phase 3: Write new METADATA
    let metadata_bytes = metadata.serialize().into_bytes();
    let metadata_hash = hash_content(&metadata_bytes);
    let metadata_size = metadata_bytes.len() as u64;

    writer.start_file(&new_metadata_path, options)?;
    writer.write_all(&metadata_bytes)?;

    new_record_entries.push(RecordEntry::new(
        new_metadata_path,
        Some(metadata_hash),
        Some(metadata_size),
    ));

    // Phase 4: Write new RECORD (RECORD itself has no hash)
    new_record_entries.push(RecordEntry::new(new_record_path.clone(), None, None));

    let record = Record {
        entries: new_record_entries,
    };
    let record_content = record.serialize();

    writer.start_file(&new_record_path, options)?;
    writer.write_all(record_content.as_bytes())?;

    // Finalize the archive
    writer.finish()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn create_test_wheel() -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();

            // Add package file
            zip.start_file("test_pkg/__init__.py", options).unwrap();
            zip.write_all(b"__version__ = '1.0.0'\n").unwrap();

            // Add METADATA
            let metadata = "Metadata-Version: 2.1\nName: test-pkg\nVersion: 1.0.0\n";
            zip.start_file("test_pkg-1.0.0.dist-info/METADATA", options)
                .unwrap();
            zip.write_all(metadata.as_bytes()).unwrap();

            // Add WHEEL
            let wheel =
                "Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: true\nTag: py3-none-any\n";
            zip.start_file("test_pkg-1.0.0.dist-info/WHEEL", options)
                .unwrap();
            zip.write_all(wheel.as_bytes()).unwrap();

            // Add RECORD
            let record = "test_pkg/__init__.py,sha256=abc,21\ntest_pkg-1.0.0.dist-info/METADATA,sha256=def,50\ntest_pkg-1.0.0.dist-info/WHEEL,sha256=ghi,70\ntest_pkg-1.0.0.dist-info/RECORD,,\n";
            zip.start_file("test_pkg-1.0.0.dist-info/RECORD", options)
                .unwrap();
            zip.write_all(record.as_bytes()).unwrap();

            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn test_write_modified_same_version() {
        let wheel_data = create_test_wheel();
        let mut source = ZipArchive::new(Cursor::new(wheel_data)).unwrap();

        let mut metadata = Metadata::default();
        metadata.metadata_version = "2.1".to_string();
        metadata.name = "test-pkg".to_string();
        metadata.version = "1.0.0".to_string();
        metadata.summary = Some("Modified summary".to_string());

        let record = Record::parse(
            "test_pkg/__init__.py,sha256=abc,21\ntest_pkg-1.0.0.dist-info/WHEEL,sha256=ghi,70\n",
        )
        .unwrap();

        let mut output = Cursor::new(Vec::new());
        write_modified(
            &mut source,
            &mut output,
            &metadata,
            &record,
            "test_pkg-1.0.0.dist-info",
            "test_pkg-1.0.0.dist-info",
        )
        .unwrap();

        // Verify output is valid ZIP
        let output_data = output.into_inner();
        let result = ZipArchive::new(Cursor::new(output_data)).unwrap();
        assert!(result.len() >= 3);
    }

    #[test]
    fn test_write_modified_version_change() {
        let wheel_data = create_test_wheel();
        let mut source = ZipArchive::new(Cursor::new(wheel_data)).unwrap();

        let mut metadata = Metadata::default();
        metadata.metadata_version = "2.1".to_string();
        metadata.name = "test-pkg".to_string();
        metadata.version = "1.0.1".to_string(); // Changed version

        let record = Record::parse(
            "test_pkg/__init__.py,sha256=abc,21\ntest_pkg-1.0.0.dist-info/WHEEL,sha256=ghi,70\n",
        )
        .unwrap();

        let mut output = Cursor::new(Vec::new());
        write_modified(
            &mut source,
            &mut output,
            &metadata,
            &record,
            "test_pkg-1.0.0.dist-info",
            "test_pkg-1.0.1.dist-info", // New dist-info name
        )
        .unwrap();

        // Verify output contains renamed files
        let output_data = output.into_inner();
        let mut result = ZipArchive::new(Cursor::new(output_data)).unwrap();

        let mut found_new_metadata = false;
        for i in 0..result.len() {
            let file = result.by_index(i).unwrap();
            if file.name() == "test_pkg-1.0.1.dist-info/METADATA" {
                found_new_metadata = true;
            }
        }
        assert!(found_new_metadata, "New METADATA path not found");
    }
}
