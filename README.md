# editwheel

High-performance Python wheel metadata editor written in Rust.

## Overview

`editwheel` provides fast editing of Python wheel metadata and ELF binaries by copying unchanged files as raw compressed bytes, only modifying the files that need to change.

This makes it ideal for scenarios where you need to quickly modify wheel metadata (e.g., version bumping), change platform tags, or patch RPATH in native extensions without the overhead of fully extracting and repacking large wheels.

## Features

- **Metadata editing**: Modify package name, version, dependencies, and other metadata fields
- **ELF patching**: Set RPATH/RUNPATH on `.so` files (similar to `patchelf`)
- **Platform tag modification**: Change wheel platform tags (e.g., `linux_x86_64` → `manylinux_2_28_x86_64`)
- **Python bindings**: Use from Python via PyO3
- **Rust library**: Use directly from Rust
- **CLI tool**: Command-line interface for quick edits
- **Full wheel validation**: Verify file hashes against RECORD
- **pip compatible**: Output wheels are fully compatible with pip and other Python tooling

## Installation

### Python

```bash
# Build and install using maturin
uv sync
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
| `platform_tag` | `str` | Platform tag from WHEEL file |

#### ELF patching (native wheels)

```python
from editwheel import WheelEditor

editor = WheelEditor("torch-2.0.0-cp311-cp311-linux_x86_64.whl")

# Set RPATH on all .so files matching a glob pattern
count = editor.set_rpath("torch/lib/*.so", "$ORIGIN:$ORIGIN/../lib")
print(f"Modified {count} files")

# Get RPATH of a specific file
rpath = editor.get_rpath("torch/lib/libtorch.so")

# Change platform tag (e.g., for manylinux compliance)
editor.platform_tag = "manylinux_2_28_x86_64"

# Add a dependency
editor.add_requires_dist("nccl-lib>=1.0")

# Check if any ELF files were modified
if editor.has_modified_files():
    print("ELF files were patched")

# Save the modified wheel
editor.save("torch-2.0.0-cp311-cp311-manylinux_2_28_x86_64.whl")
```

#### Generic metadata access

```python
# Get any metadata field
value = editor.get_metadata("Author")

# Set any metadata field
editor.set_metadata("License", "MIT")
editor.set_metadata("Classifier", ["Development Status :: 4 - Beta", "License :: OSI Approved :: MIT License"])
```

### CLI

```bash
# Show wheel metadata
editwheel show mypackage-1.0.0-py3-none-any.whl

# Show as JSON
editwheel show mypackage.whl --json

# Show specific fields
editwheel show mypackage.whl -f name -f version

# Edit version
editwheel edit mypackage.whl --version 1.0.1

# Edit and save to new file
editwheel edit mypackage.whl --author "New Author" -o modified.whl

# Add dependencies
editwheel edit mypackage.whl --add-requires-dist "click>=8.0"

# Set RPATH on native extensions
editwheel edit torch.whl --set-rpath 'torch/lib/*.so' '$ORIGIN:$ORIGIN/../lib'

# Change platform tag
editwheel edit torch.whl --platform-tag manylinux_2_28_x86_64

# Combined operations
editwheel edit torch.whl \
  --set-rpath 'torch/lib/*.so' '$ORIGIN' \
  --platform-tag manylinux_2_28_x86_64 \
  --add-requires-dist 'nccl-lib>=1.0' \
  -o modified_torch.whl
```

#### Available edit options

| Option | Description |
|--------|-------------|
| `--output`, `-o` | Output path (default: overwrite in-place) |
| `--name` | Set package name |
| `--version` | Set version |
| `--summary` | Set summary/description |
| `--author` | Set author name |
| `--author-email` | Set author email |
| `--license` | Set license |
| `--requires-python` | Set Python version requirement |
| `--add-classifier` | Add a classifier (repeatable) |
| `--set-classifiers` | Replace all classifiers (comma-separated) |
| `--add-requires-dist` | Add a dependency (repeatable) |
| `--set-requires-dist` | Replace all dependencies (comma-separated) |
| `--set-rpath PATTERN RPATH` | Set RPATH for ELF files matching pattern (repeatable) |
| `--platform-tag` | Set platform tag in WHEEL file |

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
cargo build --release

# Build Python wheel
uv sync
```

### Testing

```bash
# Run Rust tests
cargo test

# Run integration tests (downloads wheels from PyPI)
cargo test --release --test integration_test -- --nocapture

# Run Python tests
.venv/bin/pytest
```

### Benchmarking

```bash
cargo run --release --example bench_edit
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
3. Only regenerates files that need to change (`METADATA`, `WHEEL`, `RECORD`, and any patched ELF files)
4. Updates file hashes in `RECORD` for modified files

This results in near-constant-time performance regardless of wheel size. For ELF patching operations, only the affected `.so` files are decompressed, modified, and recompressed.

## License

MIT
