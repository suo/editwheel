//! Benchmark script for editwheel-rs wheel editing
//!
//! Run with:
//!   cargo run --release --example bench_edit

use std::path::Path;
use std::process::Command;
use std::time::Instant;

use editwheel::WheelEditor;

fn main() {
    let input_wheel = Path::new(
        "../builder/output/wheels/dist/torch-2.10.0a0+git1d21b4d-cp312-cp312-linux_x86_64.whl",
    );
    let output_wheel = Path::new(
        "../builder/output/wheels/dist/torch-2.10.0.2025.12.04.0+git1d21b4d-cp312-cp312-linux_x86_64.whl",
    );

    println!("=== editwheel-rs Benchmark ===\n");

    // Check input file exists
    if !input_wheel.exists() {
        eprintln!("Error: Input wheel not found at {:?}", input_wheel);
        std::process::exit(1);
    }

    // Get input file size
    let input_size = std::fs::metadata(input_wheel).map(|m| m.len()).unwrap_or(0);
    println!("Input wheel: {:?}", input_wheel);
    println!("Input size:  {:.2} MB", input_size as f64 / 1024.0 / 1024.0);
    println!();

    // Benchmark: Open wheel
    let start = Instant::now();
    let mut editor = match WheelEditor::open(input_wheel) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("Error opening wheel: {}", err);
            std::process::exit(1);
        }
    };
    let open_time = start.elapsed();
    println!("Open wheel:  {:?}", open_time);

    // Display current metadata
    println!("\nCurrent metadata:");
    println!("  Name:    {}", editor.name());
    println!("  Version: {}", editor.version());

    // Benchmark: Modify version
    let start = Instant::now();
    editor.set_version("2.10.0.2025.12.04.0+git1d21b4d");
    let modify_time = start.elapsed();
    println!("\nModify version: {:?}", modify_time);
    println!("  New version: {}", editor.version());

    // Benchmark: Save wheel
    let start = Instant::now();
    if let Err(err) = editor.save(output_wheel) {
        eprintln!("Error saving wheel: {}", err);
        std::process::exit(1);
    }
    let save_time = start.elapsed();
    println!("\nSave wheel:  {:?}", save_time);

    // Get output file size
    let output_size = std::fs::metadata(output_wheel)
        .map(|m| m.len())
        .unwrap_or(0);
    println!(
        "Output size: {:.2} MB",
        output_size as f64 / 1024.0 / 1024.0
    );

    // Total time
    let total_time = open_time + modify_time + save_time;
    println!("\n=== Summary ===");
    println!("Total time:  {:?}", total_time);
    println!(
        "Throughput:  {:.2} MB/s",
        (input_size as f64 / 1024.0 / 1024.0) / total_time.as_secs_f64()
    );
    println!("\nOutput: {:?}", output_wheel);

    // Validate with pip
    println!("\n=== Validating with pip ===");
    let output = Command::new("uvx")
        .args([
            "-p3.12",
            "pip",
            "install",
            "--dry-run",
            "--no-deps",
            output_wheel.to_str().unwrap(),
        ])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("✅ pip validation PASSED");
            } else {
                eprintln!("❌ pip validation FAILED:");
                eprintln!("{}", String::from_utf8_lossy(&result.stderr));
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to run uvx: {}", e);
            std::process::exit(1);
        }
    }
}
