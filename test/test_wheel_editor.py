#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.

"""
Unit tests for editwheel Python bindings.

Tests that wheels modified by the Rust-based wheel_editor remain valid and conform
to all Python packaging standards and tooling requirements.
"""

import base64
import csv
import hashlib
import io
import tempfile
import zipfile
from pathlib import Path

import pytest
from editwheel import normalize_dist_info_name, WheelEditor


def create_test_wheel(temp_dir: Path) -> Path:
    """Create a minimal but valid wheel file for testing."""
    wheel_path = temp_dir / "test_package-1.0.0-py3-none-any.whl"

    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as zf:
        # Create package files
        zf.writestr(
            "test_package/__init__.py", "# Test package\n__version__ = '1.0.0'\n"
        )
        zf.writestr(
            "test_package/module.py", "def hello():\n    return 'Hello, World!'\n"
        )

        # Create METADATA
        metadata = """Metadata-Version: 2.1
Name: test-package
Version: 1.0.0
Summary: A test package for wheel editor validation
Author: Test Author
Author-email: test@example.com
License: MIT
Classifier: Development Status :: 3 - Alpha
Classifier: Programming Language :: Python :: 3
Requires-Python: >=3.6
Requires-Dist: requests>=2.20.0

This is a test package created to validate the wheel editor.
It contains minimal content but follows all wheel standards.
"""
        zf.writestr("test_package-1.0.0.dist-info/METADATA", metadata)

        # Create WHEEL file
        wheel_info = """Wheel-Version: 1.0
Generator: test-wheel-creator (1.0.0)
Root-Is-Purelib: true
Tag: py3-none-any
"""
        zf.writestr("test_package-1.0.0.dist-info/WHEEL", wheel_info)

        # Create top_level.txt
        zf.writestr("test_package-1.0.0.dist-info/top_level.txt", "test_package\n")

        # Create RECORD file with hashes
        record_entries = []
        files_to_hash = [
            ("test_package/__init__.py", "# Test package\n__version__ = '1.0.0'\n"),
            (
                "test_package/module.py",
                "def hello():\n    return 'Hello, World!'\n",
            ),
            ("test_package-1.0.0.dist-info/METADATA", metadata),
            ("test_package-1.0.0.dist-info/WHEEL", wheel_info),
            ("test_package-1.0.0.dist-info/top_level.txt", "test_package\n"),
        ]

        for filename, content in files_to_hash:
            content_bytes = content.encode("utf-8")
            hash_digest = hashlib.sha256(content_bytes).digest()
            hash_str = base64.urlsafe_b64encode(hash_digest).decode("ascii").rstrip("=")
            size = len(content_bytes)
            record_entries.append([filename, f"sha256={hash_str}", str(size)])

        # RECORD file itself has empty hash
        record_entries.append(["test_package-1.0.0.dist-info/RECORD", "", ""])

        # Write RECORD
        output = io.StringIO()
        writer = csv.writer(output)
        writer.writerows(record_entries)
        zf.writestr("test_package-1.0.0.dist-info/RECORD", output.getvalue())

    return wheel_path


class TestNormalizeDistInfoName:
    """Tests for normalize_dist_info_name function."""

    def test_hyphen_to_underscore(self):
        assert normalize_dist_info_name("my-package") == "my_package"

    def test_dot_to_underscore(self):
        assert normalize_dist_info_name("my.package") == "my_package"

    def test_multiple_separators(self):
        assert normalize_dist_info_name("my-package.name") == "my_package_name"

    def test_already_normalized(self):
        assert normalize_dist_info_name("my_package") == "my_package"

    def test_mixed_case_preserved(self):
        # Case should be preserved in dist-info names
        assert normalize_dist_info_name("My-Package") == "My_Package"


class TestLoadWheel:
    """Tests for loading wheel files."""

    def test_load_valid_wheel(self):
        """Test that WheelEditor can load a valid wheel file."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            assert editor.name == "test-package"
            assert editor.version == "1.0.0"
            assert editor.summary == "A test package for wheel editor validation"
            assert editor.author == "Test Author"
            assert editor.author_email == "test@example.com"
            assert editor.license == "MIT"
            assert editor.requires_python == ">=3.6"
            assert "requests>=2.20.0" in editor.requires_dist

    def test_invalid_wheel_path_raises_error(self):
        """Test that invalid wheel paths raise appropriate errors."""
        with pytest.raises(FileNotFoundError):
            WheelEditor("/nonexistent/wheel.whl")

        with tempfile.NamedTemporaryFile(suffix=".txt") as f:
            with pytest.raises(ValueError, match=".whl"):
                WheelEditor(f.name)

    def test_repr(self):
        """Test __repr__ output."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))
            repr_str = repr(editor)

            assert "test-package" in repr_str
            assert "1.0.0" in repr_str


class TestEditMetadata:
    """Tests for editing metadata fields."""

    def test_edit_metadata_fields(self):
        """Test editing various metadata fields."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Edit simple fields
            editor.version = "1.0.1"
            editor.summary = "Modified summary"
            editor.author = "New Author"
            editor.author_email = "new@example.com"
            editor.license = "Apache-2.0"
            editor.requires_python = ">=3.7"

            # Verify changes in memory
            assert editor.version == "1.0.1"
            assert editor.summary == "Modified summary"
            assert editor.author == "New Author"
            assert editor.author_email == "new@example.com"
            assert editor.license == "Apache-2.0"
            assert editor.requires_python == ">=3.7"

    def test_edit_list_fields(self):
        """Test editing list-based metadata fields."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Edit classifiers
            classifiers = editor.classifiers
            classifiers.append("Development Status :: 4 - Beta")
            classifiers.append("Topic :: Software Development :: Testing")
            editor.classifiers = classifiers

            assert "Development Status :: 4 - Beta" in editor.classifiers
            assert "Topic :: Software Development :: Testing" in editor.classifiers

            # Edit dependencies
            deps = editor.requires_dist
            deps.append("click>=8.0.0")
            editor.requires_dist = deps

            assert "click>=8.0.0" in editor.requires_dist


class TestGetSetMetadata:
    """Tests for get_metadata and set_metadata methods."""

    def test_get_metadata_string_field(self):
        """Test getting single-value metadata fields."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            assert editor.get_metadata("Name") == "test-package"
            assert editor.get_metadata("Version") == "1.0.0"
            assert editor.get_metadata("Author") == "Test Author"

    def test_get_metadata_list_field(self):
        """Test getting multi-value metadata fields."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            classifiers = editor.get_metadata("Classifier")
            assert isinstance(classifiers, list)
            assert len(classifiers) == 2

    def test_set_metadata_string_field(self):
        """Test setting single-value metadata fields."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            editor.set_metadata("Author", "New Author")
            assert editor.get_metadata("Author") == "New Author"

    def test_set_metadata_list_field(self):
        """Test setting multi-value metadata fields."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            new_classifiers = ["License :: OSI Approved :: MIT License"]
            editor.set_metadata("Classifier", new_classifiers)
            assert editor.get_metadata("Classifier") == new_classifiers

    def test_custom_metadata_fields(self):
        """Test setting custom metadata fields."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Set custom fields
            editor.set_metadata("Home-page", "https://example.com/test")
            editor.set_metadata("Download-URL", "https://example.com/download")

            # Save and verify
            output_path = temp_path / "edited.whl"
            editor.save(str(output_path))

            new_editor = WheelEditor(str(output_path))
            assert new_editor.get_metadata("Home-page") == "https://example.com/test"
            assert (
                new_editor.get_metadata("Download-URL")
                == "https://example.com/download"
            )


class TestSaveWheel:
    """Tests for saving edited wheels."""

    def test_save_edited_wheel(self):
        """Test saving an edited wheel maintains validity."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Make edits
            editor.version = "1.0.1"
            editor.summary = "Modified test package"

            # Save to new file
            output_path = temp_path / "test_package-1.0.1-py3-none-any.whl"
            editor.save(str(output_path))

            # Verify file exists
            assert output_path.exists()
            assert output_path.stat().st_size > 0

            # Load the saved wheel and verify changes
            new_editor = WheelEditor(str(output_path))
            assert new_editor.version == "1.0.1"
            assert new_editor.summary == "Modified test package"

    def test_overwrite_original_wheel(self):
        """Test that save() without output_path overwrites the original."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Make changes
            editor.version = "1.0.1"
            editor.summary = "Overwritten wheel"

            # Save without specifying output path
            editor.save()

            # Verify the original file was modified
            assert test_wheel.exists()

            # Load and verify changes
            new_editor = WheelEditor(str(test_wheel))
            assert new_editor.version == "1.0.1"
            assert new_editor.summary == "Overwritten wheel"

    def test_record_file_updated(self):
        """Test that RECORD file is properly updated with new hashes."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Make a change that affects file content
            editor.version = "2.0.0"
            editor.description = "New description added"

            # Save edited wheel
            output_path = temp_path / "test_package-1.0.1-py3-none-any.whl"
            editor.save(str(output_path))

            # Check RECORD file in the edited wheel
            with zipfile.ZipFile(output_path, "r") as zf:
                record_files = [
                    name for name in zf.namelist() if name.endswith("/RECORD")
                ]

                assert len(record_files) == 1, "Should have exactly one RECORD file"

                with zf.open(record_files[0]) as f:
                    record_content = f.read().decode("utf-8")

                    # Parse RECORD
                    reader = csv.reader(io.StringIO(record_content))
                    records = list(reader)

                    # Verify RECORD has entries
                    assert len(records) > 0, "RECORD should not be empty"

                    # Check format of non-RECORD entries
                    for row in records:
                        if len(row) >= 3:
                            path, hash_str, size = row[0], row[1], row[2]
                            if not path.endswith("/RECORD"):
                                if hash_str:
                                    assert hash_str.startswith(
                                        "sha256="
                                    ), f"Hash should be SHA256 format for {path}"
                                if size:
                                    assert (
                                        size.isdigit()
                                    ), f"Size should be numeric for {path}"

    def test_metadata_version_preserved(self):
        """Test that Metadata-Version is preserved correctly."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))
            original_metadata_version = editor.get_metadata("Metadata-Version")

            # Edit other fields
            editor.version = "1.0.1"

            # Save
            output_path = temp_path / "edited.whl"
            editor.save(str(output_path))

            # Check metadata version is preserved
            new_editor = WheelEditor(str(output_path))
            assert (
                new_editor.get_metadata("Metadata-Version") == original_metadata_version
            )


class TestDependencyEditing:
    """Tests for editing dependencies."""

    def test_duplicate_dependency_different_version(self):
        """Test adding the same dependency with a different version creates duplicates."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Original wheel has requests>=2.20.0
            assert "requests>=2.20.0" in editor.requires_dist
            original_count = len(editor.requires_dist)

            # Add the same dependency with a different version
            deps = editor.requires_dist
            deps.append("requests>=3.0.0")
            editor.requires_dist = deps

            # Should now have both versions (duplicates)
            assert "requests>=2.20.0" in editor.requires_dist
            assert "requests>=3.0.0" in editor.requires_dist
            assert len(editor.requires_dist) == original_count + 1

            # Save and verify duplicates persist
            output_path = temp_path / "duplicate_deps.whl"
            editor.save(str(output_path))

            new_editor = WheelEditor(str(output_path))
            assert "requests>=2.20.0" in new_editor.requires_dist
            assert "requests>=3.0.0" in new_editor.requires_dist

    def test_replace_dependency_version(self):
        """Test replacing a dependency with a different version."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Original wheel has requests>=2.20.0
            assert "requests>=2.20.0" in editor.requires_dist

            # Replace the dependency with a new version
            deps = editor.requires_dist
            deps = [
                dep if not dep.startswith("requests") else "requests>=3.0.0"
                for dep in deps
            ]
            editor.requires_dist = deps

            # Should only have the new version
            assert "requests>=2.20.0" not in editor.requires_dist
            assert "requests>=3.0.0" in editor.requires_dist

            # Save and verify replacement persists
            output_path = temp_path / "replaced_deps.whl"
            editor.save(str(output_path))

            new_editor = WheelEditor(str(output_path))
            assert "requests>=2.20.0" not in new_editor.requires_dist
            assert "requests>=3.0.0" in new_editor.requires_dist

    def test_multiple_version_specifiers(self):
        """Test handling dependencies with multiple version specifiers."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Add various version specifiers for the same package
            deps = editor.requires_dist
            deps.extend(
                [
                    "requests==2.28.0",  # Exact version
                    "requests<3.0.0",  # Upper bound
                    "requests!=2.25.0",  # Not equal
                    "requests~=2.20",  # Compatible release
                ]
            )
            editor.requires_dist = deps

            # All should be present (even though they may conflict)
            assert "requests>=2.20.0" in editor.requires_dist  # Original
            assert "requests==2.28.0" in editor.requires_dist
            assert "requests<3.0.0" in editor.requires_dist
            assert "requests!=2.25.0" in editor.requires_dist
            assert "requests~=2.20" in editor.requires_dist

            # Save and verify all are preserved
            output_path = temp_path / "multi_version_deps.whl"
            editor.save(str(output_path))

            new_editor = WheelEditor(str(output_path))
            assert (
                len(
                    [
                        dep
                        for dep in new_editor.requires_dist
                        if dep.startswith("requests")
                    ]
                )
                == 5
            )

    def test_dependency_with_extras(self):
        """Test handling dependencies with extras and markers."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            editor = WheelEditor(str(test_wheel))

            # Add dependencies with various extras and markers
            deps = editor.requires_dist
            deps.extend(
                [
                    "requests[security]>=2.20.0",
                    "requests[socks]>=2.20.0",
                    'requests>=3.0.0; python_version>="3.8"',
                    'requests>=2.28.0; sys_platform=="win32"',
                ]
            )
            editor.requires_dist = deps

            # All variations should be present
            assert "requests>=2.20.0" in editor.requires_dist  # Original
            assert "requests[security]>=2.20.0" in editor.requires_dist
            assert "requests[socks]>=2.20.0" in editor.requires_dist
            assert 'requests>=3.0.0; python_version>="3.8"' in editor.requires_dist
            assert 'requests>=2.28.0; sys_platform=="win32"' in editor.requires_dist

            # Save and verify
            output_path = temp_path / "extras_deps.whl"
            editor.save(str(output_path))

            new_editor = WheelEditor(str(output_path))
            requests_deps = [
                dep for dep in new_editor.requires_dist if "requests" in dep
            ]
            assert len(requests_deps) == 5


class TestEndToEnd:
    """End-to-end tests."""

    def test_e2e(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)

            # Step 1: Create test wheel
            print("\n1. Creating test wheel...")
            wheel_path = create_test_wheel(temp_path)
            print(f"   Created: {wheel_path}")
            print(f"   Size: {wheel_path.stat().st_size:,} bytes")

            # Step 2: Edit the wheel using WheelEditor
            print("\n2. Editing wheel with WheelEditor...")
            editor = WheelEditor(str(wheel_path))

            print("   Original metadata:")
            print(f"     Name: {editor.name}")
            print(f"     Version: {editor.version}")
            print(f"     Summary: {editor.summary}")
            print(f"     Author: {editor.author}")

            # Make edits
            editor.version = "1.0.1"
            editor.summary = "Modified test package - validated by wheel editor"
            editor.author = "Wheel Editor Test Suite"
            editor.set_metadata("Home-page", "https://example.com/wheel-editor-test")

            # Add classifiers
            classifiers = editor.classifiers
            classifiers.append("Development Status :: 4 - Beta")
            classifiers.append("Topic :: Software Development :: Testing")
            editor.classifiers = classifiers

            # Add dependencies
            deps = editor.requires_dist
            deps.append("click>=8.0.0")
            editor.requires_dist = deps

            # Save edited wheel
            edited_path = temp_path / "test_package-1.0.1-py3-none-any.whl"
            editor.save(str(edited_path))
            print(f"\n   Saved edited wheel: {edited_path}")
            print(f"   Size: {edited_path.stat().st_size:,} bytes")

            # Step 3: Verify changes were applied
            print("\n3. Verifying changes...")
            new_editor = WheelEditor(str(edited_path))

            assert (
                new_editor.version == "1.0.1"
            ), f"Version not updated correctly: {new_editor.version}"
            assert (
                "validated by wheel editor" in new_editor.summary
            ), f"Summary not updated correctly: {new_editor.summary}"
            assert (
                new_editor.author == "Wheel Editor Test Suite"
            ), f"Author not updated correctly: {new_editor.author}"
            assert (
                "click>=8.0.0" in new_editor.requires_dist
            ), f"Dependencies not updated correctly: {new_editor.requires_dist}"

            print("   ✅ All changes verified successfully!")


class TestHashPreservation:
    """Tests for hash preservation in saved wheels."""

    def test_hash_preservation(self):
        """Test that unchanged files maintain their original hashes."""
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_wheel = create_test_wheel(temp_path)

            # Get original RECORD hashes
            original_hashes = {}
            with zipfile.ZipFile(test_wheel, "r") as zf:
                for name in zf.namelist():
                    if name.endswith("/RECORD"):
                        with zf.open(name) as f:
                            reader = csv.reader(io.StringIO(f.read().decode("utf-8")))
                            for row in reader:
                                if len(row) >= 2 and row[1]:
                                    original_hashes[row[0]] = row[1]

            # Edit and save
            editor = WheelEditor(str(test_wheel))
            editor.summary = "Hash preservation test"

            output_path = temp_path / "preserved.whl"
            editor.save(str(output_path))

            # Check hashes of unchanged files
            with zipfile.ZipFile(output_path, "r") as zf:
                for name in zf.namelist():
                    if name.endswith("/RECORD"):
                        with zf.open(name) as f:
                            reader = csv.reader(io.StringIO(f.read().decode("utf-8")))
                            for row in reader:
                                if len(row) >= 2 and row[1]:
                                    # Unchanged files should have same hash
                                    if not row[0].endswith("/METADATA"):
                                        if row[0] in original_hashes:
                                            assert (
                                                row[1] == original_hashes[row[0]]
                                            ), f"Hash changed for {row[0]}"
