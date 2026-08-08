# noson

`noson` is a Rust library (published on crates.io) that generates random JSON
values conforming to a given JSON Schema. The schema comes in as a
`serde_json::Value`; the caller supplies the RNG, so a seeded RNG makes output
reproducible. Its real-world consumer is a content-metadata service where an
LLM drafts schemas and noson powers a "generate example" button — the output
must pass that service's strict validator (`format` asserted, unknown formats
rejected).

## Layout

- `src/lib.rs` — public API (`generate`), crate docs with the authoritative
  list of supported/unsupported schema keywords (mirrored in `README.md`;
  keep both in sync when capabilities change).
- `src/generate.rs` — the generator itself: per-type generation, constraint
  handling, composition (`allOf`/`anyOf`/`oneOf`/`not`/`if-then-else`),
  `$ref` resolution, schema merging.
- `src/xeger.rs` — random string generation from a regex (for `pattern`),
  walking the `regex-syntax` HIR.
- `src/error.rs` — the `Error` enum.
- `tests/corpus.rs` + `tests/corpus/` — integration test running `generate`
  over ~75 real-world schemas (AI-drafted, harvested from the consuming
  service) across 200 seeds each, validating output with the `jsonschema`
  crate in the consumer's strict configuration. `EXPECTED_FAILURES` in
  `corpus.rs` lists known-red files (currently empty — the whole corpus is
  green); the test asserts both directions, so fixing a capability forces the
  corresponding line's deletion. See `tests/corpus/RECOMMENDATIONS.md` for
  corpus provenance and capability history.

## Development

Tasks are defined in `mise.toml`:

- `mise run test` — `cargo nextest run --all-targets`
- `mise run clippy` — clippy with `-D warnings`
- `mise run fmt` / `fmt:check` — nightly rustfmt
- `mise run ci` — all of the above

Releases are automated with release-plz; version bumps and the changelog are
derived from conventional-commit messages.

## Commit messages

- Use conventional commit style (`feat:`, `fix:`, `test:`, `docs:`, `chore:`, …).
- When a change breaks the public API (removing or renaming public items,
  changing signatures, adding/removing/reordering enum variants), mark the
  commit as breaking in conventional-commit style: add `!` to the type
  (`feat!:`, `fix!:`) and include a `BREAKING-CHANGE:` footer describing what
  broke and what replaces it. release-plz derives version bumps and the
  changelog from these markers; its cargo-semver-checks integration is a
  safety net, not a substitute — it only catches lintable API changes on lib
  targets.
