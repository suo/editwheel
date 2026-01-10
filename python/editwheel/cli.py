#!/usr/bin/env python3
# Copyright (c) Meta Platforms, Inc. and affiliates.

"""
CLI for editwheel - High-performance Python wheel metadata editor.

Usage:
    editwheel show <wheel>  # Display wheel metadata
    editwheel edit <wheel>  # Modify metadata fields and save
"""

import json
import sys
from typing import Optional, Tuple

import click

from editwheel.editwheel import WheelEditor


@click.group()
@click.version_option(version="0.2.0", prog_name="editwheel")
def cli() -> None:
    """High-performance Python wheel metadata editor.

    Edit wheel metadata without extracting and repacking the entire wheel.
    Achieves constant-time performance by copying unchanged files as raw
    compressed bytes.
    """
    pass


@cli.command()
@click.argument("wheel", type=click.Path(exists=True))
@click.option("--json", "as_json", is_flag=True, help="Output as JSON")
@click.option(
    "--field",
    "-f",
    multiple=True,
    help="Show only specific field(s). Can be repeated.",
)
def show(wheel: str, as_json: bool, field: Tuple[str, ...]) -> None:
    """Display wheel metadata.

    WHEEL is the path to a .whl file to inspect.

    Examples:

        editwheel show mypackage-1.0.0-py3-none-any.whl

        editwheel show mypackage.whl --json

        editwheel show mypackage.whl -f name -f version
    """
    try:
        editor = WheelEditor(wheel)
    except Exception as e:
        click.echo(f"Error: {e}", err=True)
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
        "platform_tag": editor.platform_tag,
    }

    # Filter to specific fields if requested
    if field:
        # Normalize field names (allow both underscore and hyphen)
        normalized_fields = {f.replace("-", "_").lower() for f in field}
        metadata = {
            k: v for k, v in metadata.items() if k.lower() in normalized_fields
        }
        if not metadata:
            click.echo(f"Error: No matching fields found for: {', '.join(field)}", err=True)
            sys.exit(1)

    if as_json:
        click.echo(json.dumps(metadata, indent=2))
    else:
        # Human-readable output
        for key, value in metadata.items():
            if value is None:
                continue
            if isinstance(value, list):
                if value:
                    click.echo(f"{key}:")
                    for item in value:
                        click.echo(f"  - {item}")
            else:
                click.echo(f"{key}: {value}")


@cli.command()
@click.argument("wheel", type=click.Path(exists=True))
@click.option("--output", "-o", type=click.Path(), help="Output path (default: overwrite in-place)")
@click.option("--name", "pkg_name", help="Set package name")
@click.option("--version", help="Set version")
@click.option("--summary", help="Set summary/description")
@click.option("--author", help="Set author name")
@click.option("--author-email", help="Set author email")
@click.option("--license", "pkg_license", help="Set license")
@click.option("--requires-python", help="Set Python version requirement (e.g., '>=3.8')")
@click.option(
    "--add-classifier",
    multiple=True,
    help="Add a classifier. Can be repeated.",
)
@click.option(
    "--set-classifiers",
    help="Replace all classifiers (comma-separated)",
)
@click.option(
    "--add-requires-dist",
    multiple=True,
    help="Add a dependency. Can be repeated.",
)
@click.option(
    "--set-requires-dist",
    help="Replace all dependencies (comma-separated)",
)
@click.option(
    "--set-rpath",
    nargs=2,
    multiple=True,
    metavar="PATTERN RPATH",
    help="Set RPATH for ELF files matching PATTERN. Can be repeated. Example: --set-rpath 'torch/lib/*.so' '$ORIGIN'",
)
@click.option(
    "--platform-tag",
    help="Set platform tag for the wheel (e.g., 'manylinux_2_28_x86_64')",
)
def edit(
    wheel: str,
    output: Optional[str],
    pkg_name: Optional[str],
    version: Optional[str],
    summary: Optional[str],
    author: Optional[str],
    author_email: Optional[str],
    pkg_license: Optional[str],
    requires_python: Optional[str],
    add_classifier: Tuple[str, ...],
    set_classifiers: Optional[str],
    add_requires_dist: Tuple[str, ...],
    set_requires_dist: Optional[str],
    set_rpath: Tuple[Tuple[str, str], ...],
    platform_tag: Optional[str],
) -> None:
    """Edit wheel metadata fields and save.

    WHEEL is the path to a .whl file to edit.

    Examples:

        editwheel edit mypackage.whl --version 1.0.1

        editwheel edit mypackage.whl --author "New Author" -o modified.whl

        editwheel edit mypackage.whl --add-requires-dist "click>=8.0"

        editwheel edit torch.whl --set-rpath 'torch/lib/*.so' '$ORIGIN'

        editwheel edit torch.whl --platform-tag manylinux_2_28_x86_64
    """
    try:
        editor = WheelEditor(wheel)
    except Exception as e:
        click.echo(f"Error: {e}", err=True)
        sys.exit(1)

    changes_made = False

    # Apply single-value field changes
    if pkg_name is not None:
        editor.name = pkg_name
        changes_made = True

    if version is not None:
        editor.version = version
        changes_made = True

    if summary is not None:
        editor.summary = summary
        changes_made = True

    if author is not None:
        editor.author = author
        changes_made = True

    if author_email is not None:
        editor.author_email = author_email
        changes_made = True

    if pkg_license is not None:
        editor.license = pkg_license
        changes_made = True

    if requires_python is not None:
        editor.requires_python = requires_python
        changes_made = True

    # Handle classifiers
    if set_classifiers is not None:
        editor.classifiers = [c.strip() for c in set_classifiers.split(",") if c.strip()]
        changes_made = True
    elif add_classifier:
        classifiers = list(editor.classifiers)
        classifiers.extend(add_classifier)
        editor.classifiers = classifiers
        changes_made = True

    # Handle requires_dist
    if set_requires_dist is not None:
        editor.requires_dist = [d.strip() for d in set_requires_dist.split(",") if d.strip()]
        changes_made = True
    elif add_requires_dist:
        deps = list(editor.requires_dist)
        deps.extend(add_requires_dist)
        editor.requires_dist = deps
        changes_made = True

    # Handle RPATH modifications
    if set_rpath:
        for pattern, rpath in set_rpath:
            try:
                count = editor.set_rpath(pattern, rpath)
                click.echo(f"Set RPATH on {count} file(s) matching '{pattern}'")
                if count > 0:
                    changes_made = True
            except Exception as e:
                click.echo(f"Error setting RPATH for '{pattern}': {e}", err=True)
                sys.exit(1)

    # Handle platform tag
    if platform_tag is not None:
        editor.platform_tag = platform_tag
        click.echo(f"Set platform tag to: {platform_tag}")
        changes_made = True

    if not changes_made:
        click.echo("No changes specified. Use --help to see available options.", err=True)
        sys.exit(1)

    # Save the wheel
    try:
        editor.save(output)
        if output:
            click.echo(f"Saved to: {output}")
        else:
            click.echo(f"Updated: {wheel}")
    except Exception as e:
        click.echo(f"Error saving wheel: {e}", err=True)
        sys.exit(1)


if __name__ == "__main__":
    cli()
