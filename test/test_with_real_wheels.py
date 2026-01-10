#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.

"""
End-to-end test using real wheels from PyPI.

This script downloads real wheels and tests that the Rust-based editor can modify them
while maintaining full compatibility with all Python tooling.

Usage:
    uv run pytest test/test_with_real_wheels.py -v
"""

import base64
import csv
import hashlib
import io
import subprocess
import tempfile
import urllib.request
import zipfile
from pathlib import Path

import pytest
from editwheel import WheelEditor

# Small, pure-Python wheels that are good for testing
TEST_WHEELS = [
    {
        "name": "six",
        "url": "https://files.pythonhosted.org/packages/d9/5a/e7c31adbe875f2abbb91bd84cf2dc52d792b5a01506781dbcf25c91daf11/six-1.16.0-py2.py3-none-any.whl",
        "description": "Simple pure Python compatibility library",
    },
    {
        "name": "click",
        "url": "https://files.pythonhosted.org/packages/00/2e/d53fa4befbf2cfa713304affc7ca780ce4fc1fd8710527771b58311a3229/click-8.1.7-py3-none-any.whl",
        "description": "Command line interface library",
    },
    {
        "name": "wheel",
        "url": "https://files.pythonhosted.org/packages/0b/2c/87f3254fd8ffd29e4c02732eee68a83a1d3c346ae39bc6822dcbcb697f2b/wheel-0.45.1-py3-none-any.whl",
        "description": "The wheel package itself",
    },
]


@pytest.fixture
def temp_dir():
    """Provide a temporary directory for testing."""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


def download_wheel(url: str, dest_path: Path) -> Path:
    """Download a wheel from URL."""
    print(f"Downloading: {url}")
    print(f"         to: {dest_path}")

    urllib.request.urlretrieve(url, dest_path)
    print(f"✅ Downloaded successfully ({dest_path.stat().st_size:,} bytes)")
    return dest_path


def generate_edited_wheel_filename(original_filename: str) -> str:
    """
    Generate a valid wheel filename for the edited version.

    Wheel filenames follow the format:
    {distribution}-{version}(-{build tag})?-{python}-{abi}-{platform}.whl
    """
    # Parse the original filename
    name_parts = original_filename.replace(".whl", "").split("-")

    if len(name_parts) < 5:
        # For pure Python wheels like: six-1.16.0-py2.py3-none-any.whl
        # Parts: ['six', '1.16.0', 'py2.py3', 'none', 'any']
        return original_filename.replace(".whl", "_edited.whl")

    # Modify the version part to add +edited
    name_parts[1] = f"{name_parts[1]}+edited"

    # Reconstruct the filename
    return "-".join(name_parts) + ".whl"


def validate_wheel_hashes(wheel_path: Path) -> bool:
    """Validate that all file hashes in RECORD match actual content."""
    with zipfile.ZipFile(wheel_path, "r") as zf:
        # Find RECORD file
        record_path = None
        for name in zf.namelist():
            if name.endswith("/RECORD"):
                record_path = name
                break

        if not record_path:
            return False

        with zf.open(record_path) as f:
            record_content = f.read().decode("utf-8")
            reader = csv.reader(io.StringIO(record_content))

            for row in reader:
                if len(row) < 3:
                    continue

                path, hash_str, size_str = row[0], row[1], row[2]

                # Skip RECORD itself
                if path.endswith("/RECORD"):
                    continue

                if not hash_str:
                    continue

                # Verify hash
                if not hash_str.startswith("sha256="):
                    return False

                expected_hash = hash_str[7:]  # Remove "sha256=" prefix

                try:
                    with zf.open(path) as entry:
                        content = entry.read()
                        actual_hash = (
                            base64.urlsafe_b64encode(hashlib.sha256(content).digest())
                            .decode("ascii")
                            .rstrip("=")
                        )

                        if actual_hash != expected_hash:
                            print(f"Hash mismatch for {path}")
                            return False

                        # Verify size
                        if size_str and len(content) != int(size_str):
                            print(f"Size mismatch for {path}")
                            return False
                except KeyError:
                    print(f"File not found: {path}")
                    return False

    return True


@pytest.mark.parametrize(
    "wheel_info", TEST_WHEELS, ids=[w["name"] for w in TEST_WHEELS]
)
class TestRealWheels:
    """Test editing real wheels from PyPI."""

    def test_wheel_download(self, wheel_info, temp_dir):
        """Test that we can download wheels from PyPI."""
        wheel_filename = wheel_info["url"].split("/")[-1]
        wheel_path = temp_dir / wheel_filename

        downloaded = download_wheel(wheel_info["url"], wheel_path)

        assert downloaded.exists()
        assert downloaded.stat().st_size > 0

    def test_original_wheel_valid(self, wheel_info, temp_dir):
        """Test that original wheels from PyPI are valid."""
        wheel_filename = wheel_info["url"].split("/")[-1]
        wheel_path = temp_dir / wheel_filename
        download_wheel(wheel_info["url"], wheel_path)

        is_valid = validate_wheel_hashes(wheel_path)
        assert is_valid, "Original wheel from PyPI should have valid hashes"

    def test_edit_wheel(self, wheel_info, temp_dir):
        """Test that we can edit a wheel and maintain validity."""
        # Download original wheel
        wheel_filename = wheel_info["url"].split("/")[-1]
        original_wheel = temp_dir / wheel_filename
        download_wheel(wheel_info["url"], original_wheel)

        # Validate original
        assert validate_wheel_hashes(original_wheel), "Original wheel should be valid"

        # Edit the wheel
        editor = WheelEditor(str(original_wheel))

        original_version = editor.version

        # Make modifications
        editor.version = f"{editor.version}+edited"
        if editor.summary:
            editor.summary = f"{editor.summary} (Modified by editwheel-rs test)"
        else:
            editor.summary = "Modified by editwheel-rs test"

        # Add custom metadata
        editor.set_metadata("Home-page", "https://example.com/edited")

        # Add a classifier
        classifiers = editor.classifiers
        classifiers.append("Environment :: Console")
        editor.classifiers = classifiers

        # Save edited wheel with proper filename
        edited_filename = generate_edited_wheel_filename(wheel_filename)
        edited_wheel = temp_dir / edited_filename
        editor.save(str(edited_wheel))

        assert edited_wheel.exists(), "Edited wheel should be created"

        # Validate edited wheel
        assert validate_wheel_hashes(
            edited_wheel
        ), "Edited wheel should have valid hashes"

        # Verify metadata was changed
        editor2 = WheelEditor(str(edited_wheel))
        assert (
            editor2.version == f"{original_version}+edited"
        ), "Version should be updated"
        assert editor2.summary is not None
        assert (
            "Modified by editwheel-rs test" in editor2.summary
        ), "Summary should be updated"

    def test_pip_compatibility(self, wheel_info, temp_dir):
        """Test that edited wheels are pip-compatible."""
        # Download and edit wheel
        wheel_filename = wheel_info["url"].split("/")[-1]
        original_wheel = temp_dir / wheel_filename
        download_wheel(wheel_info["url"], original_wheel)

        editor = WheelEditor(str(original_wheel))
        editor.version = f"{editor.version}+edited"

        # Save with proper filename
        edited_filename = generate_edited_wheel_filename(wheel_filename)
        edited_wheel = temp_dir / edited_filename
        editor.save(str(edited_wheel))

        # Test pip install (dry run)
        result = subprocess.run(
            [
                "python",
                "-m",
                "pip",
                "install",
                "--dry-run",
                "--no-deps",
                str(edited_wheel),
            ],
            capture_output=True,
            text=True,
        )

        pip_valid = result.returncode == 0 or "Would install" in result.stdout

        assert (
            pip_valid
        ), f"Edited wheel should be pip-compatible. Error: {result.stderr}"


if __name__ == "__main__":
    # Allow running with python directly
    pytest.main([__file__, "-v"])
