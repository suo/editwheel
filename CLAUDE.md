# CLAUDE.md

## Releasing a new version

1. Bump the version in both `pyproject.toml` and `Cargo.toml`
2. Add a new entry to `CHANGELOG.md` with the release date and a summary of changes since the last tag (`git log v<previous>..HEAD --oneline`)
3. Commit with message `bump to <version>`
4. Create a git tag `v<version>` and push both the commit and the tag
