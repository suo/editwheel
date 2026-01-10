# editwheel

High-performance Python wheel metadata editor written in Rust.

## Overview

`editwheel` provides constant-time editing of Python wheel metadata, regardless of wheel size. It achieves this by copying unchanged files as raw compressed bytes, only modifying the `METADATA` and `RECORD` files.

This makes it ideal for scenarios where you need to quickly modify wheel metadata (e.g., version bumping) without the overhead of fully extracting and repacking large wheels.

## Features

- **Constant-time performance**: Edit metadata in O(1) time regardless of wheel size
- **Python bindings**: Use from Python via PyO3
- **Rust library**: Use directly from Rust
- **Full wheel validation**: Verify file hashes against RECORD
- **pip compatible**: Output wheels are fully compatible with pip and other Python tooling

## Installation

### Python

```bash
# Build and install using maturin
with-proxy uv sync
```

### Rust

Add to your `Cargo.toml`:

```toml
[dependencies]
editwheel = { path = "path/to/editwheel" }
```

## Usage

### Python

```python
from editwheel import WheelEditor

# Open a wheel
editor = WheelEditor("package-1.0.0-py3-none-any.whl")

# Read metadata
print(f"Name: {editor.name}")
print(f"Version: {editor.version}")

# Modify metadata
editor.version = "1.0.1"
editor.summary = "Updated package summary"
editor.requires_dist = ["requests>=2.0", "numpy"]

# Save to new file
editor.save("package-1.0.1-py3-none-any.whl")

# Or overwrite in place
editor.save()
```

#### Available properties

| Property | Type | Description |
|----------|------|-------------|
| `name` | `str` | Package name |
| `version` | `str` | Package version |
| `summary` | `str` | Short description |
| `description` | `str` | Long description |
| `author` | `str` | Author name |
| `author_email` | `str` | Author email |
| `license` | `str` | License identifier |
| `requires_python` | `str` | Python version requirement |
| `classifiers` | `list[str]` | Trove classifiers |
| `requires_dist` | `list[str]` | Dependencies |
| `project_urls` | `list[str]` | Project URLs |

#### Generic metadata access

```python
# Get any metadata field
value = editor.get_metadata("Author")

# Set any metadata field
editor.set_metadata("License", "MIT")
editor.set_metadata("Classifier", ["Development Status :: 4 - Beta", "License :: OSI Approved :: MIT License"])
```

### Rust

```rust
use editwheel::WheelEditor;

fn main() -> Result<(), editwheel::WheelError> {
    // Open a wheel
    let mut editor = WheelEditor::open("package-1.0.0-py3-none-any.whl")?;

    // Read metadata
    println!("Name: {}", editor.name());
    println!("Version: {}", editor.version());

    // Modify metadata
    editor.set_version("1.0.1");
    editor.set_summary("Updated summary");

    // Validate wheel integrity
    let result = editor.validate()?;
    assert!(result.is_valid());

    // Save to new file
    editor.save("package-1.0.1-py3-none-any.whl")?;

    Ok(())
}
```

## Development

### Prerequisites

- Rust 1.70+
- Python 3.8+
- [uv](https://github.com/astral-sh/uv) (recommended) or pip

### Building

```bash
# Build Rust library
with-proxy cargo build --release

# Build Python wheel
with-proxy uv sync
```

### Testing

```bash
# Run Rust tests
with-proxy cargo test

# Run integration tests (downloads wheels from PyPI)
with-proxy cargo test --release --test integration_test -- --nocapture

# Run Python tests
.venv/bin/pytest
```

### Benchmarking

```bash
with-proxy cargo run --release --example bench_edit
```

## How it works

Traditional wheel editing requires:
1. Extracting all files from the wheel (ZIP archive)
2. Modifying metadata files
3. Re-compressing all files back into a new wheel

For large wheels (e.g., PyTorch at ~1GB), this is slow and memory-intensive.

`editwheel` instead:
1. Opens the wheel as a ZIP archive
2. Copies unchanged files using raw compressed bytes (no decompression/recompression)
3. Only regenerates `METADATA` and `RECORD` files
4. Updates file hashes in `RECORD` for modified files

This results in constant-time performance regardless of wheel size.

## License

MIT
