# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-04-29

### Added

- `WheelEditor.add_file(path, content)` to inject brand-new files into the wheel archive (e.g. a `build-details.json` stamped into `.dist-info/`). Paths under the old dist-info / `.data` prefix are rewritten when the wheel is renamed, and collisions with existing source files or generated dist-info files (METADATA/RECORD/WHEEL) are rejected at save time.
- `WheelEditor.dist_info_dir` getter returning the dist-info directory name as it would appear in the saved wheel — useful for constructing paths to pass to `add_file`.
- `WheelEditor.validate()` Python binding (previously Rust-only). Returns a `ValidationResult` with `is_valid` and `errors` (list of human-readable strings); supports `bool(result)` as a shortcut for `result.is_valid`.
- `editwheel edit --add-file ARCHIVE_PATH SRC` and `editwheel edit --add-dist-info-file FILENAME SRC` to expose the new file-injection API on the CLI. `editwheel show` now also reports `dist_info_dir`.
- `editwheel validate WHEEL [--json]` CLI subcommand exiting non-zero on validation errors.

## [0.2.5] - 2026-02-19

### Fixed

- Rename `.data` directories when rewriting wheel versions to keep them consistent with the new version

## [0.2.4] - 2026-02-10

### Added

- `python_tag` and `abi_tag` setters for wheel tag editing
- `filename()` method to get the canonical wheel filename from metadata

### Changed

- Migrated CLI from click to argparse, removing the only runtime dependency

## [0.2.3] - 2026-01-14

### Fixed

- Make page size platform aware for ELF patching (fixes issues on non-4K page size systems like ARM64)

## [0.2.2] - 2025-01-10

### Fixed

- Enable ZIP64 support for large files (>4GB) when writing modified wheels

## [0.2.1] - 2025-01-10

### Fixed

- Fixed PyO3 deprecation warnings: use `Py<PyAny>` instead of `PyObject`
- Added README.md as PyPI project description

## [0.2.0] - 2025-01-09

### Added

- **ELF patching support**: Set RPATH/RUNPATH on `.so` files inside wheels
  - `set_rpath(pattern, rpath)` method to modify RPATH using glob patterns
  - `get_rpath(path)` method to read current RPATH of a file
  - `has_modified_files()` method to check if any ELF files were modified
  - Uses the `elb` crate for pure-Rust ELF patching (no external dependencies)

- **Platform tag modification**: Change wheel platform tags in the WHEEL file
  - `platform_tag` property to get/set the platform tag
  - Useful for converting wheels between platform formats (e.g., `linux_x86_64` to `manylinux_2_28_x86_64`)

- **CLI enhancements**:
  - `--set-rpath PATTERN RPATH` option to set RPATH on matching files
  - `--platform-tag TAG` option to change the wheel's platform tag
  - Platform tag now shown in `editwheel show` output

- **New Python API methods**:
  - `add_requires_dist(dep)` convenience method for adding dependencies
  - `has_modified_files()` to check for pending ELF modifications

- **Integration tests** for ELF patching with real native wheels from PyPI

### Changed

- Wheel writer now uses extended format when WHEEL file or ELF files are modified
- Updated documentation with ELF patching examples

## [0.1.0] - 2024-12-01

### Added

- Initial release
- High-performance wheel metadata editing using raw ZIP copy
- Python bindings via PyO3
- CLI tool with `show` and `edit` commands
- Full wheel validation with hash verification
- Support for all standard metadata fields
- Generic `get_metadata`/`set_metadata` for arbitrary fields
