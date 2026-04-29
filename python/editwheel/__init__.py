# Copyright (c) Meta Platforms, Inc. and affiliates.

"""
editwheel - High-performance Python wheel metadata editor.

This module provides a fast way to edit Python wheel metadata without
extracting and repacking the entire wheel. It achieves constant-time
performance by copying unchanged files as raw compressed bytes.
"""

# Re-export from the Rust extension module
from editwheel.editwheel import (
    ValidationResult,
    WheelEditor,
    normalize_dist_info_name,
)

__all__ = ["ValidationResult", "WheelEditor", "normalize_dist_info_name"]
