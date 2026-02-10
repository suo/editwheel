#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.

"""
CLI for editwheel - High-performance Python wheel metadata editor.

Usage:
    editwheel show <wheel>  # Display wheel metadata
    editwheel edit <wheel>  # Modify metadata fields and save
"""

import argparse
import json
import os
import sys
from importlib.metadata import version as _pkg_version
from typing import List, Optional

from editwheel.editwheel import WheelEditor

__version__ = _pkg_version("editwheel")


def _existing_path(path: str) -> str:
    """Argparse type that validates the path exists."""
    if not os.path.exists(path):
        raise argparse.ArgumentTypeError(f"path '{path}' does not exist")
    return path


def _show(args: argparse.Namespace) -> None:
    """Handle the 'show' subcommand."""
    wheel = args.wheel

    try:
        editor = WheelEditor(wheel)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    # Build metadata dict
    metadata = {
        "name": editor.name,
        "version": editor.version,
        "summary": editor.summary,
        "author": editor.author,
        "author_email": editor.author_email,
        "license": editor.license,
        "requires_python": editor.requires_python,
        "classifiers": editor.classifiers,
        "requires_dist": editor.requires_dist,
        "project_urls": editor.project_urls,
        "python_tag": editor.python_tag,
        "abi_tag": editor.abi_tag,
        "platform_tag": editor.platform_tag,
    }

    # Filter to specific fields if requested
    if args.field:
        # Normalize field names (allow both underscore and hyphen)
        normalized_fields = {f.replace("-", "_").lower() for f in args.field}
        metadata = {
            k: v for k, v in metadata.items() if k.lower() in normalized_fields
        }
        if not metadata:
            print(
                f"Error: No matching fields found for: {', '.join(args.field)}",
                file=sys.stderr,
            )
            sys.exit(1)

    if args.as_json:
        print(json.dumps(metadata, indent=2))
    else:
        # Human-readable output
        for key, value in metadata.items():
            if value is None:
                continue
            if isinstance(value, list):
                if value:
                    print(f"{key}:")
                    for item in value:
                        print(f"  - {item}")
            else:
                print(f"{key}: {value}")


def _edit(args: argparse.Namespace) -> None:
    """Handle the 'edit' subcommand."""
    wheel = args.wheel

    try:
        editor = WheelEditor(wheel)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    changes_made = False

    # Apply single-value field changes
    if args.pkg_name is not None:
        editor.name = args.pkg_name
        changes_made = True

    if args.version is not None:
        editor.version = args.version
        changes_made = True

    if args.summary is not None:
        editor.summary = args.summary
        changes_made = True

    if args.author is not None:
        editor.author = args.author
        changes_made = True

    if args.author_email is not None:
        editor.author_email = args.author_email
        changes_made = True

    if args.pkg_license is not None:
        editor.license = args.pkg_license
        changes_made = True

    if args.requires_python is not None:
        editor.requires_python = args.requires_python
        changes_made = True

    # Handle classifiers
    if args.set_classifiers is not None:
        editor.classifiers = [
            c.strip() for c in args.set_classifiers.split(",") if c.strip()
        ]
        changes_made = True
    elif args.add_classifier:
        classifiers = list(editor.classifiers)
        classifiers.extend(args.add_classifier)
        editor.classifiers = classifiers
        changes_made = True

    # Handle requires_dist
    if args.set_requires_dist is not None:
        editor.requires_dist = [
            d.strip() for d in args.set_requires_dist.split(",") if d.strip()
        ]
        changes_made = True
    elif args.add_requires_dist:
        deps = list(editor.requires_dist)
        deps.extend(args.add_requires_dist)
        editor.requires_dist = deps
        changes_made = True

    # Handle RPATH modifications
    if args.set_rpath:
        for pattern, rpath in args.set_rpath:
            try:
                count = editor.set_rpath(pattern, rpath)
                print(f"Set RPATH on {count} file(s) matching '{pattern}'")
                if count > 0:
                    changes_made = True
            except Exception as e:
                print(f"Error setting RPATH for '{pattern}': {e}", file=sys.stderr)
                sys.exit(1)

    # Handle platform tag
    if args.platform_tag is not None:
        editor.platform_tag = args.platform_tag
        print(f"Set Platform tag to: {args.platform_tag}")
        changes_made = True

    # Handle python tag
    if args.python_tag is not None:
        editor.python_tag = args.python_tag
        print(f"Set Python tag to: {args.python_tag}")
        changes_made = True

    # Handle ABI tag
    if args.abi_tag is not None:
        editor.abi_tag = args.abi_tag
        print(f"Set ABI tag to: {args.abi_tag}")
        changes_made = True

    if not changes_made:
        print(
            "No changes specified. Use --help to see available options.", file=sys.stderr
        )
        sys.exit(1)

    # Save the wheel
    output = args.output
    try:
        if output and os.path.isdir(output):
            output = os.path.join(output, editor.filename)
        editor.save(output)
        if output:
            print(f"Saved to: {output}")
        else:
            print(f"Updated: {wheel}")
    except Exception as e:
        print(f"Error saving wheel: {e}", file=sys.stderr)
        sys.exit(1)


def _build_parser() -> argparse.ArgumentParser:
    """Build and return the argument parser."""
    parser = argparse.ArgumentParser(
        prog="editwheel",
        description=(
            "High-performance Python wheel metadata editor.\n\n"
            "Edit wheel metadata without extracting and repacking the entire wheel.\n"
            "Achieves constant-time performance by copying unchanged files as raw\n"
            "compressed bytes."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--version", action="version", version=f"editwheel {__version__}"
    )

    subparsers = parser.add_subparsers(dest="command", required=True)

    # --- show subcommand ---
    show_parser = subparsers.add_parser(
        "show",
        help="Display wheel metadata",
        description="Display wheel metadata.",
    )
    show_parser.add_argument("wheel", type=_existing_path, help="Path to a .whl file to inspect")
    show_parser.add_argument(
        "--json", dest="as_json", action="store_true", help="Output as JSON"
    )
    show_parser.add_argument(
        "--field",
        "-f",
        action="append",
        help="Show only specific field(s). Can be repeated.",
    )

    # --- edit subcommand ---
    edit_parser = subparsers.add_parser(
        "edit",
        help="Edit wheel metadata fields and save",
        description="Edit wheel metadata fields and save.",
    )
    edit_parser.add_argument("wheel", type=_existing_path, help="Path to a .whl file to edit")
    edit_parser.add_argument(
        "--output",
        "-o",
        help="Output path or directory (default: overwrite in-place)",
    )
    edit_parser.add_argument("--name", dest="pkg_name", help="Set package name")
    edit_parser.add_argument("--version", help="Set version")
    edit_parser.add_argument("--summary", help="Set summary/description")
    edit_parser.add_argument("--author", help="Set author name")
    edit_parser.add_argument("--author-email", help="Set author email")
    edit_parser.add_argument("--license", dest="pkg_license", help="Set license")
    edit_parser.add_argument(
        "--requires-python",
        help="Set Python version requirement (e.g., '>=3.8')",
    )
    edit_parser.add_argument(
        "--add-classifier",
        action="append",
        default=[],
        help="Add a classifier. Can be repeated.",
    )
    edit_parser.add_argument(
        "--set-classifiers",
        help="Replace all classifiers (comma-separated)",
    )
    edit_parser.add_argument(
        "--add-requires-dist",
        action="append",
        default=[],
        help="Add a dependency. Can be repeated.",
    )
    edit_parser.add_argument(
        "--set-requires-dist",
        help="Replace all dependencies (comma-separated)",
    )
    edit_parser.add_argument(
        "--set-rpath",
        nargs=2,
        action="append",
        default=[],
        metavar=("PATTERN", "RPATH"),
        help=(
            "Set RPATH for ELF files matching PATTERN. Can be repeated. "
            "Example: --set-rpath 'torch/lib/*.so' '$ORIGIN'"
        ),
    )
    edit_parser.add_argument(
        "--platform-tag",
        help="Set platform tag for the wheel (e.g., 'manylinux_2_28_x86_64')",
    )
    edit_parser.add_argument(
        "--python-tag",
        help="Set python tag for the wheel (e.g., 'cp312')",
    )
    edit_parser.add_argument(
        "--abi-tag",
        help="Set ABI tag for the wheel (e.g., 'cp312')",
    )

    return parser


def cli(args: Optional[List[str]] = None) -> None:
    """Main CLI entrypoint.

    Args:
        args: Command-line arguments. If None, uses sys.argv.
    """
    parser = _build_parser()
    parsed = parser.parse_args(args)

    if parsed.command == "show":
        _show(parsed)
    elif parsed.command == "edit":
        _edit(parsed)


if __name__ == "__main__":
    cli()
