# CLAUDE.md

## Feature parity: library and CLI

Every user-facing capability in editwheel must be reachable from **both**
the library API (`WheelEditor` / Rust + Python) **and** the CLI
(`editwheel show` / `editwheel edit`). Default to shipping any new
feature in both surfaces in the same change — e.g. when adding
`add_file` to `WheelEditor`, also add `--add-file` (and any reasonable
convenience variant) to `editwheel edit`, and surface any new derived
state in `editwheel show`.

When a feature genuinely makes no sense in one surface, document why
in the CHANGELOG entry.

## Releasing a new version

1. Bump the version in both `pyproject.toml` and `Cargo.toml`
2. Add a new entry to `CHANGELOG.md` with the release date and a summary of changes since the last tag (`git log v<previous>..HEAD --oneline`)
3. Commit with message `bump to <version>`
4. Create a git tag `v<version>` and push both the commit and the tag
