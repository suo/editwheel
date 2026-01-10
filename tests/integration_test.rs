//! Integration tests for editwheel-rs using real wheels from PyPI.
//!
//! These tests download real wheels and verify that editwheel-rs can modify them
//! while maintaining full compatibility with Python tooling.
//!
//! Run with:
//!   cargo test --release --test integration_test -- --nocapture

use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use editwheel::WheelEditor;
use editwheel::hash_content;
use tempfile::TempDir;

/// Test wheel info from PyPI
struct TestWheel {
    name: &'static str,
    url: &'static str,
    #[allow(dead_code)]
    description: &'static str,
}

const TEST_WHEELS: &[TestWheel] = &[
    TestWheel {
        name: "six",
        url: "https://files.pythonhosted.org/packages/d9/5a/e7c31adbe875f2abbb91bd84cf2dc52d792b5a01506781dbcf25c91daf11/six-1.16.0-py2.py3-none-any.whl",
        description: "Simple pure Python compatibility library",
    },
    TestWheel {
        name: "click",
        url: "https://files.pythonhosted.org/packages/00/2e/d53fa4befbf2cfa713304affc7ca780ce4fc1fd8710527771b58311a3229/click-8.1.7-py3-none-any.whl",
        description: "Command line interface library",
    },
    TestWheel {
        name: "wheel",
        url: "https://files.pythonhosted.org/packages/0b/2c/87f3254fd8ffd29e4c02732eee68a83a1d3c346ae39bc6822dcbcb697f2b/wheel-0.45.1-py3-none-any.whl",
        description: "The wheel package itself",
    },
];

/// Download a wheel from URL to destination path
fn download_wheel(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading: {}", url);
    println!("         to: {}", dest.display());

    let output = Command::new("curl")
        .args(["-fsSL", "-o", dest.to_str().unwrap(), url])
        .output()?;

    if !output.status.success() {
        return Err(format!("curl failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    let size = std::fs::metadata(dest)?.len();
    println!("✅ Downloaded successfully ({} bytes)", size);
    Ok(())
}

/// Generate edited wheel filename
fn generate_edited_wheel_filename(original: &str) -> String {
    let name = original.trim_end_matches(".whl");
    let parts: Vec<&str> = name.split('-').collect();

    if parts.len() < 5 {
        return format!("{}_edited.whl", name);
    }

    // Modify the version part to add +edited
    let new_version = format!("{}+edited", parts[1]);
    let mut new_parts: Vec<&str> = parts.clone();
    new_parts[1] = &new_version;
    format!("{}.whl", new_parts.join("-"))
}

/// Validate wheel ZIP structure
fn validate_zip_structure(wheel_path: &Path) -> Result<(), String> {
    let file = File::open(wheel_path).map_err(|e| format!("Failed to open wheel: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid ZIP structure: {}", e))?;

    // Check all files can be read
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry {}: {}", i, e))?;

        // Check compression type is valid
        match file.compression() {
            zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated => {}
            other => {
                return Err(format!(
                    "Invalid compression type for {}: {:?}",
                    file.name(),
                    other
                ));
            }
        }
    }

    println!("✓ ZIP structure valid");
    Ok(())
}

/// Validate METADATA file format
fn validate_metadata_format(wheel_path: &Path) -> Result<(), String> {
    let file = File::open(wheel_path).map_err(|e| format!("Failed to open wheel: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid ZIP structure: {}", e))?;

    // Find METADATA file
    let metadata_path = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().to_string();
            if name.contains(".dist-info/") && name.ends_with("/METADATA") {
                Some(name)
            } else {
                None
            }
        })
        .next()
        .ok_or("No METADATA file found")?;

    let mut metadata_file = archive
        .by_name(&metadata_path)
        .map_err(|e| format!("Failed to read METADATA: {}", e))?;

    let mut content = String::new();
    metadata_file
        .read_to_string(&mut content)
        .map_err(|e| format!("Failed to read METADATA content: {}", e))?;

    // Check required fields
    let required = ["Metadata-Version:", "Name:", "Version:"];
    let missing: Vec<_> = required
        .iter()
        .filter(|&field| !content.contains(field))
        .collect();

    if !missing.is_empty() {
        return Err(format!("Missing required METADATA fields: {:?}", missing));
    }

    println!("✓ METADATA file valid with required fields");
    Ok(())
}

/// Validate RECORD file and verify hashes
fn validate_record_file(wheel_path: &Path) -> Result<(), String> {
    let file = File::open(wheel_path).map_err(|e| format!("Failed to open wheel: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid ZIP structure: {}", e))?;

    // Get all file names in the archive
    let wheel_files: HashSet<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    // Find RECORD file
    let record_path = wheel_files
        .iter()
        .find(|name| name.contains(".dist-info/") && name.ends_with("/RECORD"))
        .ok_or("No RECORD file found")?
        .clone();

    let mut record_file = archive
        .by_name(&record_path)
        .map_err(|e| format!("Failed to read RECORD: {}", e))?;

    let mut record_content = String::new();
    record_file
        .read_to_string(&mut record_content)
        .map_err(|e| format!("Failed to read RECORD content: {}", e))?;
    drop(record_file);

    // Parse RECORD as CSV
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(record_content.as_bytes());

    let mut record_paths: HashSet<String> = HashSet::new();
    let mut errors: Vec<String> = Vec::new();
    let mut verified_count = 0;

    for result in reader.records() {
        let record = result.map_err(|e| format!("Failed to parse RECORD CSV: {}", e))?;

        if record.len() != 3 {
            errors.push(format!("Invalid row format: {:?}", record));
            continue;
        }

        let path = &record[0];
        let hash_str = &record[1];
        let size_str = &record[2];

        record_paths.insert(path.to_string());

        if path.ends_with("/RECORD") {
            // RECORD file itself should have empty hash and size
            if !hash_str.is_empty() || !size_str.is_empty() {
                errors.push("RECORD file should have empty hash/size".to_string());
            }
            continue;
        }

        // Verify file exists in wheel
        if !wheel_files.contains(path) {
            errors.push(format!("File in RECORD not found in wheel: {}", path));
            continue;
        }

        // Verify hash format
        if !hash_str.is_empty() && !hash_str.starts_with("sha256=") {
            errors.push(format!("Invalid hash format for {}: {}", path, hash_str));
            continue;
        }

        // Compute actual hash and size
        // Need to reopen archive to read file content
        let file2 = File::open(wheel_path).map_err(|e| format!("Failed to open wheel: {}", e))?;
        let reader2 = BufReader::new(file2);
        let mut archive2 =
            zip::ZipArchive::new(reader2).map_err(|e| format!("Invalid ZIP structure: {}", e))?;

        let mut entry = archive2
            .by_name(path)
            .map_err(|e| format!("Failed to read {}: {}", path, e))?;

        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .map_err(|e| format!("Failed to read content of {}: {}", path, e))?;

        let actual_size = content.len();
        let actual_hash_full = hash_content(&content); // Returns "sha256=..."
        let actual_hash = actual_hash_full
            .strip_prefix("sha256=")
            .unwrap_or(&actual_hash_full);

        // Verify hash matches (hash_content returns "sha256=..." format)
        if !hash_str.is_empty() {
            let expected_hash = hash_str.strip_prefix("sha256=").unwrap_or(hash_str);
            if expected_hash != actual_hash {
                errors.push(format!(
                    "{}: hash mismatch - expected {}, got {}",
                    path, expected_hash, actual_hash
                ));
            }
        }

        // Verify size matches
        if !size_str.is_empty() {
            let expected_size: usize = size_str
                .parse()
                .map_err(|e| format!("Invalid size for {}: {}", path, e))?;
            if expected_size != actual_size {
                errors.push(format!(
                    "{}: size mismatch - expected {}, got {}",
                    path, expected_size, actual_size
                ));
            }
        }

        verified_count += 1;
    }

    // Check for files in wheel not in RECORD
    let missing_from_record: Vec<_> = wheel_files.difference(&record_paths).collect();
    if !missing_from_record.is_empty() {
        errors.push(format!(
            "Files in wheel not in RECORD: {:?}",
            missing_from_record
        ));
    }

    if !errors.is_empty() {
        return Err(format!("RECORD validation errors: {}", errors.join("; ")));
    }

    println!(
        "✓ RECORD file valid ({} entries, {} hashes verified)",
        record_paths.len(),
        verified_count
    );
    Ok(())
}

/// Validate WHEEL file format
fn validate_wheel_file(wheel_path: &Path) -> Result<(), String> {
    let file = File::open(wheel_path).map_err(|e| format!("Failed to open wheel: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid ZIP structure: {}", e))?;

    // Find WHEEL file
    let wheel_info_path = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index(i).ok()?;
            let name = file.name().to_string();
            if name.contains(".dist-info/") && name.ends_with("/WHEEL") {
                Some(name)
            } else {
                None
            }
        })
        .next()
        .ok_or("No WHEEL file found")?;

    let mut wheel_file = archive
        .by_name(&wheel_info_path)
        .map_err(|e| format!("Failed to read WHEEL: {}", e))?;

    let mut content = String::new();
    wheel_file
        .read_to_string(&mut content)
        .map_err(|e| format!("Failed to read WHEEL content: {}", e))?;

    // Check required fields
    let required = ["Wheel-Version:", "Generator:", "Root-Is-Purelib:"];
    let missing: Vec<_> = required
        .iter()
        .filter(|&field| !content.contains(field))
        .collect();

    if !missing.is_empty() {
        return Err(format!("Missing required WHEEL fields: {:?}", missing));
    }

    println!("✓ WHEEL file valid with required fields");
    Ok(())
}

/// Validate dist-info directory structure
fn validate_dist_info_structure(wheel_path: &Path) -> Result<(), String> {
    let file = File::open(wheel_path).map_err(|e| format!("Failed to open wheel: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Invalid ZIP structure: {}", e))?;

    // Find all dist-info directories
    let mut dist_info_dirs: HashSet<String> = HashSet::new();
    for i in 0..archive.len() {
        let file = archive.by_index_raw(i).unwrap();
        let name = file.name().to_string();
        if name.contains(".dist-info/") {
            let dir = name.split(".dist-info/").next().unwrap().to_string() + ".dist-info/";
            dist_info_dirs.insert(dir);
        }
    }

    if dist_info_dirs.is_empty() {
        return Err("No .dist-info directory found".to_string());
    }

    if dist_info_dirs.len() > 1 {
        return Err(format!(
            "Multiple .dist-info directories: {:?}",
            dist_info_dirs
        ));
    }

    let dist_info_dir = dist_info_dirs.into_iter().next().unwrap();

    // Check for required files
    let required_files = ["METADATA", "WHEEL", "RECORD"];
    let dist_info_files: HashSet<String> = (0..archive.len())
        .filter_map(|i| {
            let file = archive.by_index_raw(i).ok()?;
            let name = file.name().to_string();
            if name.starts_with(&dist_info_dir) {
                Some(name.replace(&dist_info_dir, ""))
            } else {
                None
            }
        })
        .collect();

    let missing: Vec<_> = required_files
        .iter()
        .filter(|&f| !dist_info_files.contains(*f))
        .collect();

    if !missing.is_empty() {
        return Err(format!("Missing required dist-info files: {:?}", missing));
    }

    println!(
        "✓ dist-info structure valid with {} files",
        dist_info_files.len()
    );
    Ok(())
}

/// Test pip compatibility (dry run)
fn test_pip_compatibility(wheel_path: &Path) -> Result<(), String> {
    let output = Command::new("python")
        .args([
            "-m",
            "pip",
            "install",
            "--dry-run",
            "--no-deps",
            wheel_path.to_str().unwrap(),
        ])
        .output();

    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);

            if result.status.success() || stdout.contains("Would install") {
                println!("✓ pip compatibility: pip can process this wheel");
                Ok(())
            } else {
                Err(format!("pip error: {}", stderr))
            }
        }
        Err(_) => {
            println!("⚪ pip compatibility: pip not accessible");
            Ok(())
        }
    }
}

/// Validate wheel using Python's wheel tool
fn test_wheel_tool(wheel_path: &Path) -> Result<(), String> {
    let temp_dir = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let output = Command::new("python")
        .args([
            "-m",
            "wheel",
            "unpack",
            wheel_path.to_str().unwrap(),
            "-d",
            temp_dir.path().to_str().unwrap(),
        ])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("✓ wheel tool: Successfully unpacked");
                Ok(())
            } else {
                Err(format!(
                    "wheel unpack failed: {}",
                    String::from_utf8_lossy(&result.stderr)
                ))
            }
        }
        Err(_) => {
            println!("⚪ wheel tool: Not installed or not accessible");
            Ok(())
        }
    }
}

/// Run all validation checks on a wheel
fn validate_wheel_full(wheel_path: &Path) -> Result<(), String> {
    println!("\n{}", "=".repeat(70));
    println!("Validating wheel: {}", wheel_path.display());
    println!("{}\n", "=".repeat(70));

    validate_zip_structure(wheel_path)?;
    validate_metadata_format(wheel_path)?;
    validate_record_file(wheel_path)?;
    validate_wheel_file(wheel_path)?;
    validate_dist_info_structure(wheel_path)?;
    test_wheel_tool(wheel_path)?;
    test_pip_compatibility(wheel_path)?;

    println!("\n✅ All validation checks passed!");
    Ok(())
}

#[test]
fn test_download_and_validate_wheels() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    for wheel_info in TEST_WHEELS {
        println!("\n\n### Testing wheel: {} ###\n", wheel_info.name);

        let filename = wheel_info.url.split('/').last().unwrap();
        let wheel_path = temp_dir.path().join(filename);

        // Download wheel
        download_wheel(wheel_info.url, &wheel_path).expect("Failed to download wheel");

        // Validate original wheel
        validate_wheel_full(&wheel_path).expect("Original wheel should be valid");
    }
}

#[test]
fn test_edit_and_validate_wheels() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    for wheel_info in TEST_WHEELS {
        println!("\n\n### Testing wheel editing: {} ###\n", wheel_info.name);

        let filename = wheel_info.url.split('/').last().unwrap();
        let wheel_path = temp_dir.path().join(filename);

        // Download wheel
        download_wheel(wheel_info.url, &wheel_path).expect("Failed to download wheel");

        // Validate original
        validate_wheel_full(&wheel_path).expect("Original wheel should be valid");

        // Edit the wheel
        let mut editor = WheelEditor::open(&wheel_path).expect("Failed to open wheel");

        let original_version = editor.version().to_string();
        println!("\nOriginal version: {}", original_version);

        // Make modifications
        let new_version = format!("{}+edited", original_version);
        editor.set_version(&new_version);

        if let Some(summary) = editor.summary() {
            editor.set_summary(format!("{} (Modified by editwheel-rs test)", summary));
        }

        // Save edited wheel
        let edited_filename = generate_edited_wheel_filename(filename);
        let edited_path = temp_dir.path().join(&edited_filename);
        editor
            .save(&edited_path)
            .expect("Failed to save edited wheel");

        println!("\nEdited wheel saved to: {}", edited_path.display());

        // Validate edited wheel
        validate_wheel_full(&edited_path).expect("Edited wheel should be valid");

        // Verify metadata was changed
        let editor2 = WheelEditor::open(&edited_path).expect("Failed to open edited wheel");
        assert_eq!(editor2.version(), new_version, "Version should be updated");
        println!(
            "\n✅ Wheel {} edited and validated successfully!",
            wheel_info.name
        );
    }
}

#[test]
fn test_pip_compatibility_after_edit() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    for wheel_info in TEST_WHEELS {
        println!(
            "\n\n### Testing pip compatibility: {} ###\n",
            wheel_info.name
        );

        let filename = wheel_info.url.split('/').last().unwrap();
        let wheel_path = temp_dir.path().join(filename);

        // Download wheel
        download_wheel(wheel_info.url, &wheel_path).expect("Failed to download wheel");

        // Edit the wheel
        let mut editor = WheelEditor::open(&wheel_path).expect("Failed to open wheel");
        let new_version = format!("{}+edited", editor.version());
        editor.set_version(&new_version);

        // Save edited wheel
        let edited_filename = generate_edited_wheel_filename(filename);
        let edited_path = temp_dir.path().join(&edited_filename);
        editor
            .save(&edited_path)
            .expect("Failed to save edited wheel");

        // Test pip compatibility
        test_pip_compatibility(&edited_path).expect("Edited wheel should be pip compatible");

        println!(
            "\n✅ Wheel {} is pip compatible after editing!",
            wheel_info.name
        );
    }
}
