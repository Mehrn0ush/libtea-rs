## What and why

<!-- One concern per pull request. -->

## Checklist

- [ ] The gate in CONTRIBUTING.md passed locally (fmt, clippy `-D warnings`, tests on stable and 1.89.0, doctests, rustdoc, guards, deny)
- [ ] Tests added or changed, and they fail without the change
- [ ] Docs and `CHANGELOG.md` updated where behavior changed
- [ ] Every commit is signed off (`git commit -s`)
- [ ] Actions and tools in workflows are pinned (SHA with a `# vX.Y.Z` comment, exact versions)
- [ ] No credentials in the diff, the description or logs
