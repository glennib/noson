# Commit messages

- Use conventional commit style (`feat:`, `fix:`, `test:`, `docs:`, `chore:`, …).
- When a change breaks the public API (removing or renaming public items,
  changing signatures, adding/removing/reordering enum variants), mark the
  commit as breaking in conventional-commit style: add `!` to the type
  (`feat!:`, `fix!:`) and include a `BREAKING-CHANGE:` footer describing what
  broke and what replaces it. release-plz derives version bumps and the
  changelog from these markers; its cargo-semver-checks integration is a
  safety net, not a substitute — it only catches lintable API changes on lib
  targets.
