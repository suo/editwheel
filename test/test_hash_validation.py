#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.

"""Test script to verify RECORD file hash validation in editwheel-rs."""

import base64
import csv
import hashlib
import io
import zipfile
from pathlib import Path

import pytest
from editwheel import WheelEditor


def create_test_wheel_with_valid_hashes(temp_dir: Path) -> Path:
    """Create a test wheel with valid hashes in RECORD."""
    wheel_path = temp_dir / "test_package-1.0.0-py3-none-any.whl"

    # Create wheel content
    files_to_add = {}

    # Package files
    files_to_add["test_package/__init__.py"] = (
        b"# Test package\n__version__ = '1.0.0'\n"
    )
    files_to_add["test_package/module.py"] = (
        b"def hello():\n    return 'Hello, World!'\n"
    )

    # Dist-info files
    metadata_content = b"""Metadata-Version: 2.1
Name: test-package
Version: 1.0.0
Summary: A test package
Author: Test Author
Author-email: test@example.com
"""
    files_to_add["test_package-1.0.0.dist-info/METADATA"] = metadata_content

    wheel_content = b"""Wheel-Version: 1.0
Generator: test-script
Root-Is-Purelib: true
Tag: py3-none-any
"""
    files_to_add["test_package-1.0.0.dist-info/WHEEL"] = wheel_content

    # Create RECORD with correct hashes
    record_entries = []
    for path, content in files_to_add.items():
        # Compute SHA256 hash
        hasher = hashlib.sha256()
        hasher.update(content)
        hash_digest = (
            base64.urlsafe_b64encode(hasher.digest()).decode("ascii").rstrip("=")
        )
        record_entries.append([path, f"sha256={hash_digest}", str(len(content))])

    # RECORD file itself has empty hash/size
    record_entries.append(["test_package-1.0.0.dist-info/RECORD", "", ""])

    # Create RECORD content
    output = io.StringIO()
    writer = csv.writer(output)
    writer.writerows(record_entries)
    files_to_add["test_package-1.0.0.dist-info/RECORD"] = output.getvalue().encode(
        "utf-8"
    )

    # Create wheel
    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for path, content in files_to_add.items():
            zf.writestr(path, content)

    return wheel_path


def create_test_wheel_with_invalid_hashes(temp_dir: Path) -> Path:
    """Create a test wheel with invalid hashes in RECORD."""
    wheel_path = temp_dir / "bad_package-1.0.0-py3-none-any.whl"

    # Create wheel content
    files_to_add = {}

    # Package files
    files_to_add["bad_package/__init__.py"] = b"# Bad package\n__version__ = '1.0.0'\n"
    files_to_add["bad_package/module.py"] = (
        b"def hello():\n    return 'Hello, World!'\n"
    )

    # Dist-info files
    metadata_content = b"""Metadata-Version: 2.1
Name: bad-package
Version: 1.0.0
Summary: A bad test package
"""
    files_to_add["bad_package-1.0.0.dist-info/METADATA"] = metadata_content

    wheel_content = b"""Wheel-Version: 1.0
Generator: test-script
Root-Is-Purelib: true
Tag: py3-none-any
"""
    files_to_add["bad_package-1.0.0.dist-info/WHEEL"] = wheel_content

    # Create RECORD with INCORRECT hashes (using wrong hash values)
    record_entries = [
        ["bad_package/__init__.py", "sha256=WRONGHASH123", "42"],
        ["bad_package/module.py", "sha256=BADHASH456", "37"],
        ["bad_package-1.0.0.dist-info/METADATA", "sha256=INVALIDHASH789", "85"],
        ["bad_package-1.0.0.dist-info/WHEEL", "sha256=FAKEHASH000", "69"],
        ["bad_package-1.0.0.dist-info/RECORD", "", ""],
    ]

    # Create RECORD content
    output = io.StringIO()
    writer = csv.writer(output)
    writer.writerows(record_entries)
    files_to_add["bad_package-1.0.0.dist-info/RECORD"] = output.getvalue().encode(
        "utf-8"
    )

    # Create wheel
    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for path, content in files_to_add.items():
            zf.writestr(path, content)

    return wheel_path


class TestHashValidation:
    """Tests for hash validation in wheels."""

    def test_valid_wheel_can_be_loaded(self, tmp_path):
        """Test that a wheel with valid hashes can be loaded."""
        wheel_path = create_test_wheel_with_valid_hashes(tmp_path)

        # Should not raise any exception
        editor = WheelEditor(str(wheel_path))
        assert editor.name == "test-package"
        assert editor.version == "1.0.0"

    def test_invalid_hashes_wheel_can_still_be_loaded(self, tmp_path):
        """Test that a wheel with invalid hashes can still be loaded.

        Note: The Rust library loads wheels even with invalid hashes.
        Hash validation is done separately via the validate() method.
        """
        wheel_path = create_test_wheel_with_invalid_hashes(tmp_path)

        # Should load successfully (validation is not done on load)
        editor = WheelEditor(str(wheel_path))
        assert editor.name == "bad-package"
        assert editor.version == "1.0.0"

    def test_edited_wheel_has_valid_hashes(self, tmp_path):
        """Test that an edited wheel has correct hashes."""
        wheel_path = create_test_wheel_with_valid_hashes(tmp_path)

        editor = WheelEditor(str(wheel_path))
        editor.summary = "Modified summary"

        output_path = tmp_path / "edited.whl"
        editor.save(str(output_path))

        # Verify the saved wheel has valid hashes
        with zipfile.ZipFile(output_path, "r") as zf:
            # Find RECORD
            record_path = None
            for name in zf.namelist():
                if name.endswith("/RECORD"):
                    record_path = name
                    break

            assert record_path is not None, "RECORD file should exist"

            with zf.open(record_path) as f:
                record_content = f.read().decode("utf-8")
                reader = csv.reader(io.StringIO(record_content))

                for row in reader:
                    if len(row) < 3:
                        continue

                    path, hash_str, size_str = row[0], row[1], row[2]

                    # Skip RECORD itself
                    if path.endswith("/RECORD"):
                        assert hash_str == "", "RECORD should have empty hash"
                        continue

                    if not hash_str:
                        continue

                    # Verify hash
                    assert hash_str.startswith(
                        "sha256="
                    ), f"Invalid hash format for {path}"
                    expected_hash = hash_str[7:]  # Remove "sha256=" prefix

                    with zf.open(path) as entry:
                        content = entry.read()
                        actual_hash = (
                            base64.urlsafe_b64encode(hashlib.sha256(content).digest())
                            .decode("ascii")
                            .rstrip("=")
                        )

                        assert actual_hash == expected_hash, f"Hash mismatch for {path}"

                    # Verify size
                    if size_str:
                        with zf.open(path) as entry:
                            content = entry.read()
                            assert len(content) == int(
                                size_str
                            ), f"Size mismatch for {path}"
