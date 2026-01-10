#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.

"""
Test uv pip install compatibility for edited wheels.

This script downloads a torch wheel from PyPI, edits the version, and verifies
that the edited wheel can be installed with `uv pip install`.

Usage:
    uv run pytest test/test_uv_compatibility.py -v

    # Or run directly as a script:
    uv run python test/test_uv_compatibility.py [wheel_path]

Known Issue:
    Large wheels (>4GB) using zip64 format may fail with uv due to incorrect
    zip64 extended information field handling in the raw copy logic. The error:

        zip64 extended information field was too long: expected 16 bytes, but 8
        bytes were provided

    This is a bug in the wheel writing code that needs to be fixed.
"""

import subprocess
import sys
import tempfile
from pathlib import Path

import pytest
from editwheel import WheelEditor

# PyTorch wheel to download from PyPI for testing
# Using a CUDA wheel to test with larger, more complex wheels
TORCH_PACKAGE = "torch==2.5.1+cu124"
TORCH_INDEX_URL = "https://download.pytorch.org/whl/cu124"
# Python version to use for testing (must match the downloaded wheel)
TEST_PYTHON_VERSION = "3.13"


def generate_edited_wheel_filename(original_path: Path, new_version: str) -> Path:
    """Generate proper wheel filename for edited version."""
    # Parse wheel filename: {name}-{version}-{python}-{abi}-{platform}.whl
    original_name = original_path.name
    parts = original_name.replace(".whl", "").split("-")

    # parts[0] = name, parts[1] = version, rest = python-abi-platform
    parts[1] = new_version
    new_filename = "-".join(parts) + ".whl"

    return original_path.parent / new_filename


def download_torch_wheel(dest_dir: Path) -> Path:
    """Download a torch wheel from PyPI to the destination directory.

    Returns the path to the downloaded wheel file.
    """
    # Use uv run pip download to fetch the wheel (uv doesn't have a native download command)
    # We download for the current platform since cross-platform download is tricky
    result = subprocess.run(
        [
            "uv",
            "run",
            "--no-project",
            "--with",
            "pip",
            "pip",
            "download",
            TORCH_PACKAGE,
            "--index-url",
            TORCH_INDEX_URL,
            "--only-binary=:all:",
            "--no-deps",
            "-d",
            str(dest_dir),
        ],
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        raise RuntimeError(
            f"Failed to download torch wheel:\n"
            f"stdout: {result.stdout}\n"
            f"stderr: {result.stderr}"
        )

    # Find the downloaded wheel file
    wheels = list(dest_dir.glob("torch-*.whl"))
    if not wheels:
        raise RuntimeError(f"No torch wheel found in {dest_dir}")

    return wheels[0]


@pytest.fixture(scope="module")
def torch_wheel(tmp_path_factory) -> Path:
    """Fixture that downloads a torch wheel once per test module."""
    import os
    if os.environ.get("CI"):
        pytest.skip("Torch wheel download tests disabled in CI")
    download_dir = tmp_path_factory.mktemp("wheels")
    wheel_path = download_torch_wheel(download_dir)
    print(f"\nDownloaded wheel: {wheel_path}")
    print(f"Size: {wheel_path.stat().st_size:,} bytes")
    return wheel_path


def test_uv_install_edited_wheel(tmp_path: Path, torch_wheel: Path) -> None:
    """Test that an edited wheel can be installed with uv pip install."""
    wheel_path = torch_wheel

    # Edit the wheel
    editor = WheelEditor(str(wheel_path))
    original_version = editor.version
    new_version = "99.0.0+test"
    editor.version = new_version

    # Save to temp location with proper filename
    edited_wheel = generate_edited_wheel_filename(
        tmp_path / wheel_path.name, new_version
    )
    editor.save(str(edited_wheel))

    assert edited_wheel.exists(), f"Edited wheel should exist: {edited_wheel}"
    print(f"\nOriginal wheel: {wheel_path}")
    print(f"Original version: {original_version}")
    print(f"Edited wheel: {edited_wheel}")
    print(f"New version: {new_version}")
    print(f"Edited wheel size: {edited_wheel.stat().st_size:,} bytes")

    # Create a clean uv project with Python 3.13 for testing (matches downloaded wheel)
    test_project = tmp_path / "test_project"
    test_project.mkdir()
    subprocess.run(
        ["uv", "init", "--python", TEST_PYTHON_VERSION],
        cwd=test_project,
        capture_output=True,
        check=True,
    )
    # Create the venv to ensure uv pip uses the correct Python
    subprocess.run(
        ["uv", "sync"],
        cwd=test_project,
        capture_output=True,
        check=True,
    )

    # Test uv pip install (dry-run to avoid actually installing the large wheel)
    result = subprocess.run(
        [
            "uv",
            "pip",
            "install",
            "--dry-run",
            str(edited_wheel),
        ],
        cwd=test_project,
        capture_output=True,
        text=True,
    )

    print("\nuv pip install --dry-run output:")
    print(f"stdout: {result.stdout}")
    print(f"stderr: {result.stderr}")
    print(f"return code: {result.returncode}")

    assert result.returncode == 0, (
        f"uv pip install failed for edited wheel.\n"
        f"stdout: {result.stdout}\n"
        f"stderr: {result.stderr}"
    )


def test_pip_vs_uv_comparison(tmp_path: Path, torch_wheel: Path) -> None:
    """Compare pip and uv behavior for the same edited wheel."""
    wheel_path = torch_wheel

    # Edit the wheel
    editor = WheelEditor(str(wheel_path))
    new_version = "99.0.0+comparison"
    editor.version = new_version

    edited_wheel = generate_edited_wheel_filename(
        tmp_path / wheel_path.name, new_version
    )
    editor.save(str(edited_wheel))

    # Create a clean uv project with Python 3.13 for testing (matches downloaded wheel)
    test_project = tmp_path / "test_project"
    test_project.mkdir()
    subprocess.run(
        ["uv", "init", "--python", TEST_PYTHON_VERSION],
        cwd=test_project,
        capture_output=True,
        check=True,
    )
    # Create the venv to ensure uv pip uses the correct Python
    subprocess.run(
        ["uv", "sync"],
        cwd=test_project,
        capture_output=True,
        check=True,
    )

    # Test with pip (install pip first, then use uv run pip)
    subprocess.run(
        ["uv", "add", "pip", "--dev"],
        cwd=test_project,
        capture_output=True,
        check=True,
    )
    pip_result = subprocess.run(
        [
            "uv",
            "run",
            "pip",
            "install",
            "--dry-run",
            "--no-deps",
            str(edited_wheel),
        ],
        cwd=test_project,
        capture_output=True,
        text=True,
    )
    pip_ok = pip_result.returncode == 0 or "Would install" in pip_result.stdout

    # Test with uv
    uv_result = subprocess.run(
        [
            "uv",
            "pip",
            "install",
            "--dry-run",
            str(edited_wheel),
        ],
        cwd=test_project,
        capture_output=True,
        text=True,
    )

    print("\n=== pip dry-run ===")
    print(f"return code: {pip_result.returncode}")
    print(f"stdout: {pip_result.stdout}")
    print(f"stderr: {pip_result.stderr}")

    print("\n=== uv pip dry-run ===")
    print(f"return code: {uv_result.returncode}")
    print(f"stdout: {uv_result.stdout}")
    print(f"stderr: {uv_result.stderr}")

    assert pip_ok, f"pip failed: {pip_result.stderr}"
    assert uv_result.returncode == 0, f"uv failed: {uv_result.stderr}"


def main():
    """Run as standalone script."""
    if len(sys.argv) > 1:
        wheel_path = Path(sys.argv[1]).resolve()
        if not wheel_path.exists():
            print(f"Error: Wheel not found: {wheel_path}")
            sys.exit(1)
    else:
        # Download a torch wheel from PyPI
        print("Downloading torch wheel from PyPI...")
        with tempfile.TemporaryDirectory() as download_dir:
            wheel_path = download_torch_wheel(Path(download_dir))
            _run_test_with_wheel(wheel_path)
            return

    _run_test_with_wheel(wheel_path)


def _run_test_with_wheel(wheel_path: Path) -> None:
    """Run the uv compatibility test with a given wheel."""
    print(f"Testing wheel: {wheel_path}")
    print(f"Size: {wheel_path.stat().st_size:,} bytes")

    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)

        # Edit the wheel
        editor = WheelEditor(str(wheel_path))
        original_version = editor.version
        new_version = "99.0.0+uvtest"

        print(f"\nOriginal version: {original_version}")
        print(f"Setting version to: {new_version}")

        editor.version = new_version

        # Generate proper output filename
        edited_wheel = generate_edited_wheel_filename(
            tmp_path / wheel_path.name, new_version
        )
        editor.save(str(edited_wheel))

        print(f"Saved edited wheel: {edited_wheel}")
        print(f"Edited wheel size: {edited_wheel.stat().st_size:,} bytes")

        # Create a clean uv project for testing
        test_project = tmp_path / "test_project"
        test_project.mkdir()

        print(f"\n=== Creating test environment with Python {TEST_PYTHON_VERSION} ===")
        subprocess.run(
            ["uv", "init", "--python", TEST_PYTHON_VERSION],
            cwd=test_project,
            capture_output=True,
            check=True,
        )
        # Create the venv to ensure uv pip uses it
        subprocess.run(
            ["uv", "sync"],
            cwd=test_project,
            capture_output=True,
            check=True,
        )
        print(f"Created uv project with Python {TEST_PYTHON_VERSION}")

        # Test uv pip install in the project
        print(f"\n=== Testing with uv pip install (Python {TEST_PYTHON_VERSION}) ===")
        uv_result = subprocess.run(
            [
                "uv",
                "pip",
                "install",
                "--dry-run",
                str(edited_wheel),
            ],
            cwd=test_project,
            capture_output=True,
            text=True,
        )
        print(f"uv return code: {uv_result.returncode}")
        if uv_result.stdout:
            print(f"uv stdout: {uv_result.stdout}")
        if uv_result.stderr:
            print(f"uv stderr: {uv_result.stderr}")

        # Summary
        print("\n=== Summary ===")
        uv_ok = uv_result.returncode == 0

        print(f"uv: {'PASS' if uv_ok else 'FAIL'}")

        if not uv_ok:
            print("\nUV FAILED! This indicates a compatibility issue with uv.")
            sys.exit(1)
        else:
            print("\nuv works with the edited wheel!")
            sys.exit(0)


if __name__ == "__main__":
    main()
