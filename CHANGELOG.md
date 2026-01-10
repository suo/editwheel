# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
