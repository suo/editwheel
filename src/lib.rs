//! editwheel-rs: High-performance Python wheel metadata editor
//!
//! This library provides constant-time editing of Python wheel metadata,
//! regardless of wheel size. It achieves this by copying unchanged files
//! as raw compressed bytes, only modifying METADATA and RECORD files.
//!
//! # Example
//!
//! ```no_run
//! use editwheel::WheelEditor;
//!
//! // Open a wheel
//! let mut editor = WheelEditor::open("package-1.0.0-py3-none-any.whl").unwrap();
//!
//! // Modify version
//! editor.set_version("1.0.1");
//!
//! // Save to new file
//! editor.save("package-1.0.1-py3-none-any.whl").unwrap();
//! ```

pub mod elf;
pub mod error;
pub mod metadata;
pub mod name;
pub mod record;
pub mod wheel;
pub mod wheel_info;

#[cfg(feature = "python")]
mod python;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

pub use elf::ElfInfo;
pub use elf::ElfModification;
pub use error::ElfError;
pub use error::MetadataError;
pub use error::RecordError;
pub use error::ValidationError;
pub use error::ValidationResult;
pub use error::WheelError;
pub use error::WheelInfoError;
pub use metadata::Metadata;
pub use name::dist_info_name;
pub use name::normalize_dist_info_name;
pub use record::Record;
pub use record::RecordEntry;
pub use record::hash_content;
pub use wheel::WheelReader;
pub use wheel::validate_wheel;
pub use wheel::write_modified;
pub use wheel::write_modified_extended;
pub use wheel_info::WheelInfo;
pub use wheel_info::WheelTag;

/// High-level API for editing Python wheel files
///
/// This struct provides a convenient interface for reading, modifying,
/// and saving wheel files with constant-time performance.
pub struct WheelEditor {
    path: PathBuf,
    metadata: Metadata,
    record: Record,
    dist_info_prefix: String,
    wheel_info: WheelInfo,
    /// Files that have been modified (path -> new content)
    modified_files: HashMap<String, Vec<u8>>,
    /// Whether the wheel_info has been modified (e.g., platform tag changed)
    wheel_info_modified: bool,
}

impl WheelEditor {
    /// Open a wheel file for editing
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WheelError> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut wheel_reader = WheelReader::new(reader)?;

        let metadata = wheel_reader.read_metadata()?;
        let record = wheel_reader.read_record()?;
        let wheel_info = wheel_reader.read_wheel_info()?;
        let dist_info_prefix = wheel_reader.dist_info_prefix().to_string();

        Ok(Self {
            path,
            metadata,
            record,
            dist_info_prefix,
            wheel_info,
            modified_files: HashMap::new(),
            wheel_info_modified: false,
        })
    }

    /// Get the path to the wheel file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Set the package name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.metadata.name = name.into();
    }

    /// Get the package version
    pub fn version(&self) -> &str {
        &self.metadata.version
    }

    /// Set the package version
    pub fn set_version(&mut self, version: impl Into<String>) {
        self.metadata.version = version.into();
    }

    /// Get the package summary
    pub fn summary(&self) -> Option<&str> {
        self.metadata.summary.as_deref()
    }

    /// Set the package summary
    pub fn set_summary(&mut self, summary: impl Into<String>) {
        self.metadata.summary = Some(summary.into());
    }

    /// Get the package description
    pub fn description(&self) -> Option<&str> {
        self.metadata.description.as_deref()
    }

    /// Set the package description
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.metadata.description = Some(description.into());
    }

    /// Get the package author
    pub fn author(&self) -> Option<&str> {
        self.metadata.author.as_deref()
    }

    /// Set the package author
    pub fn set_author(&mut self, author: impl Into<String>) {
        self.metadata.author = Some(author.into());
    }

    /// Get the author email
    pub fn author_email(&self) -> Option<&str> {
        self.metadata.author_email.as_deref()
    }

    /// Set the author email
    pub fn set_author_email(&mut self, email: impl Into<String>) {
        self.metadata.author_email = Some(email.into());
    }

    /// Get the package license
    pub fn license(&self) -> Option<&str> {
        self.metadata.license.as_deref()
    }

    /// Set the package license
    pub fn set_license(&mut self, license: impl Into<String>) {
        self.metadata.license = Some(license.into());
    }

    /// Get the Python version requirement
    pub fn requires_python(&self) -> Option<&str> {
        self.metadata.requires_python.as_deref()
    }

    /// Set the Python version requirement
    pub fn set_requires_python(&mut self, version: impl Into<String>) {
        self.metadata.requires_python = Some(version.into());
    }

    /// Get the package classifiers
    pub fn classifiers(&self) -> &[String] {
        &self.metadata.classifiers
    }

    /// Set the package classifiers
    pub fn set_classifiers(&mut self, classifiers: Vec<String>) {
        self.metadata.classifiers = classifiers;
    }

    /// Add a classifier
    pub fn add_classifier(&mut self, classifier: impl Into<String>) {
        self.metadata.classifiers.push(classifier.into());
    }

    /// Get the package dependencies
    pub fn requires_dist(&self) -> &[String] {
        &self.metadata.requires_dist
    }

    /// Set the package dependencies
    pub fn set_requires_dist(&mut self, deps: Vec<String>) {
        self.metadata.requires_dist = deps;
    }

    /// Add a dependency
    pub fn add_requires_dist(&mut self, dep: impl Into<String>) {
        self.metadata.requires_dist.push(dep.into());
    }

    /// Get the project URLs
    pub fn project_urls(&self) -> &[String] {
        &self.metadata.project_url
    }

    /// Set the project URLs
    pub fn set_project_urls(&mut self, urls: Vec<String>) {
        self.metadata.project_url = urls;
    }

    /// Add a project URL
    pub fn add_project_url(&mut self, url: impl Into<String>) {
        self.metadata.project_url.push(url.into());
    }

    /// Get access to the full metadata
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Get mutable access to the full metadata
    pub fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.metadata
    }

    /// Get access to the wheel info (WHEEL file)
    pub fn wheel_info(&self) -> &WheelInfo {
        &self.wheel_info
    }

    /// Get mutable access to the wheel info
    pub fn wheel_info_mut(&mut self) -> &mut WheelInfo {
        &mut self.wheel_info
    }

    /// Get the primary platform tag
    pub fn platform_tag(&self) -> Option<&str> {
        self.wheel_info.platform()
    }

    /// Set the platform tag for all tags in the wheel
    ///
    /// This modifies the WHEEL file to change the platform (e.g., from
    /// "linux_x86_64" to "manylinux_2_28_x86_64").
    pub fn set_platform_tag(&mut self, platform: &str) {
        self.wheel_info.set_platform(platform);
        self.wheel_info_modified = true;
    }

    /// Get the RPATH of a specific file in the wheel
    ///
    /// Returns the effective RPATH (prefers RUNPATH over RPATH).
    /// Returns an error if the file is not found or is not a valid ELF.
    pub fn get_rpath(&self, path: &str) -> Result<Option<String>, WheelError> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;

        let mut entry = archive
            .by_name(path)
            .map_err(|_| WheelError::Elf(error::ElfError::FileNotFound(path.to_string())))?;

        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;

        elf::get_rpath(&content).map_err(WheelError::from)
    }

    /// Set the RPATH for files matching a glob pattern
    ///
    /// This modifies all ELF files in the wheel that match the given glob pattern.
    /// Returns the number of files modified.
    ///
    /// # Example
    /// ```no_run
    /// use editwheel::WheelEditor;
    ///
    /// let mut editor = WheelEditor::open("torch-2.0.0-cp311-cp311-linux_x86_64.whl").unwrap();
    /// let count = editor.set_rpath("torch/lib/*.so", "$ORIGIN:$ORIGIN/../lib").unwrap();
    /// println!("Modified {} files", count);
    /// ```
    pub fn set_rpath(&mut self, pattern: &str, rpath: &str) -> Result<usize, WheelError> {
        let glob_pattern = glob::Pattern::new(pattern)?;

        // Open the archive to find matching files
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;

        // Find all files matching the pattern
        let mut matching_files = Vec::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if glob_pattern.matches(&name) {
                matching_files.push(name);
            }
        }

        // Modify each matching file
        let mut modified_count = 0;
        for file_path in matching_files {
            // Read the file content
            let mut entry = archive.by_name(&file_path)?;
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            drop(entry); // Release borrow

            // Check if it's an ELF file (magic bytes: 0x7F 'E' 'L' 'F')
            if content.len() < 4 || &content[0..4] != b"\x7FELF" {
                continue; // Skip non-ELF files
            }

            // Modify the ELF file - use RUNPATH (preferred over RPATH)
            let modifications = vec![ElfModification::SetRunpath(rpath.to_string())];
            match elf::modify_elf(&content, &modifications) {
                Ok(modified_content) => {
                    self.modified_files.insert(file_path, modified_content);
                    modified_count += 1;
                }
                Err(e) => {
                    // Log or handle error - for now, skip files that can't be modified
                    eprintln!("Warning: Failed to modify {}: {}", file_path, e);
                }
            }
        }

        Ok(modified_count)
    }

    /// Check if any files have been modified
    pub fn has_modified_files(&self) -> bool {
        !self.modified_files.is_empty()
    }

    /// Get the paths of all modified files
    pub fn modified_file_paths(&self) -> Vec<&str> {
        self.modified_files.keys().map(|s| s.as_str()).collect()
    }

    /// Validate all file hashes in the wheel
    ///
    /// This reads and hashes every file in the wheel to verify integrity.
    /// Note: This is NOT constant-time - it's O(wheel_size).
    pub fn validate(&self) -> Result<ValidationResult, WheelError> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;
        validate_wheel(&mut archive, &self.record)
    }

    /// Save the modified wheel to a new file
    ///
    /// This achieves constant-time performance by copying unchanged files
    /// as raw compressed bytes. Modified files (METADATA, RECORD, and any
    /// ELF files with changed RPATH) are rewritten with new content.
    pub fn save(&self, output_path: impl AsRef<Path>) -> Result<(), WheelError> {
        let output_path = output_path.as_ref();

        // Compute new dist-info prefix if name or version changed
        let new_dist_info = dist_info_name(&self.metadata.name, &self.metadata.version);

        // Open source for reading
        let source_file = File::open(&self.path)?;
        let source_reader = BufReader::new(source_file);
        let mut source_archive = zip::ZipArchive::new(source_reader)?;

        // Create output file
        let output_file = File::create(output_path)?;

        // Use extended writer if we have modified files or wheel info changes
        if !self.modified_files.is_empty() || self.wheel_info_modified {
            // Use extended writer which handles modified files and WHEEL file updates
            write_modified_extended(
                &mut source_archive,
                output_file,
                &self.metadata,
                &self.record,
                &self.dist_info_prefix,
                &new_dist_info,
                &self.modified_files,
                Some(&self.wheel_info),
            )?;
        } else {
            // Use the original writer for backward compatibility
            write_modified(
                &mut source_archive,
                output_file,
                &self.metadata,
                &self.record,
                &self.dist_info_prefix,
                &new_dist_info,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    fn create_test_wheel(dir: &Path) -> PathBuf {
        let wheel_path = dir.join("test_pkg-1.0.0-py3-none-any.whl");
        let file = File::create(&wheel_path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        // Package file
        let init_content = b"__version__ = '1.0.0'\n";
        zip.start_file("test_pkg/__init__.py", options).unwrap();
        zip.write_all(init_content).unwrap();
        let init_hash = hash_content(init_content);

        // METADATA
        let metadata =
            "Metadata-Version: 2.1\nName: test-pkg\nVersion: 1.0.0\nSummary: Test package\n";
        zip.start_file("test_pkg-1.0.0.dist-info/METADATA", options)
            .unwrap();
        zip.write_all(metadata.as_bytes()).unwrap();
        let metadata_hash = hash_content(metadata.as_bytes());

        // WHEEL
        let wheel_info =
            "Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: true\nTag: py3-none-any\n";
        zip.start_file("test_pkg-1.0.0.dist-info/WHEEL", options)
            .unwrap();
        zip.write_all(wheel_info.as_bytes()).unwrap();
        let wheel_hash = hash_content(wheel_info.as_bytes());

        // RECORD
        let record = format!(
            "test_pkg/__init__.py,{},{}\ntest_pkg-1.0.0.dist-info/METADATA,{},{}\ntest_pkg-1.0.0.dist-info/WHEEL,{},{}\ntest_pkg-1.0.0.dist-info/RECORD,,\n",
            init_hash,
            init_content.len(),
            metadata_hash,
            metadata.len(),
            wheel_hash,
            wheel_info.len()
        );
        zip.start_file("test_pkg-1.0.0.dist-info/RECORD", options)
            .unwrap();
        zip.write_all(record.as_bytes()).unwrap();

        zip.finish().unwrap();
        wheel_path
    }

    #[test]
    fn test_open_wheel() {
        let temp_dir = TempDir::new().unwrap();
        let wheel_path = create_test_wheel(temp_dir.path());

        let editor = WheelEditor::open(&wheel_path).unwrap();
        assert_eq!(editor.name(), "test-pkg");
        assert_eq!(editor.version(), "1.0.0");
        assert_eq!(editor.summary(), Some("Test package"));
    }

    #[test]
    fn test_modify_and_save() {
        let temp_dir = TempDir::new().unwrap();
        let wheel_path = create_test_wheel(temp_dir.path());
        let output_path = temp_dir.path().join("test_pkg-1.0.1-py3-none-any.whl");

        let mut editor = WheelEditor::open(&wheel_path).unwrap();
        editor.set_version("1.0.1");
        editor.set_summary("Updated summary");
        editor.save(&output_path).unwrap();

        // Verify the output
        let new_editor = WheelEditor::open(&output_path).unwrap();
        assert_eq!(new_editor.version(), "1.0.1");
        assert_eq!(new_editor.summary(), Some("Updated summary"));
    }

    #[test]
    fn test_validate() {
        let temp_dir = TempDir::new().unwrap();
        let wheel_path = create_test_wheel(temp_dir.path());

        let editor = WheelEditor::open(&wheel_path).unwrap();
        let result = editor.validate().unwrap();
        assert!(result.is_valid());
    }
}
