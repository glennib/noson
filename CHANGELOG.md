# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1](https://github.com/glennib/noson/compare/v0.2.0...v0.2.1) - 2026-08-08

### Added

- support the `maxContains` keyword
- support the `not` keyword
- support the `contains` and `minContains` keywords

### Other

- Merge pull request #25 from glennib/renovate/thiserror-2.x-lockfile
- record the closed corpus frontier
- update test corpus

## [0.2.0](https://github.com/glennib/noson/compare/v0.1.6...v0.2.0) - 2026-08-07

### Added

- support the `if`/`then`/`else` keywords
- support the `dependentRequired` keyword
- support the `prefixItems` keyword
- support the `multipleOf` keyword
- make object property selection count-aware, synthesize map entries
- support the `uniqueItems` keyword
- add format generators for uuid, email, uri, hostname, ipv4, ipv6
- [**breaking**] combine composition keywords with their siblings
- support `type` as an array of type names

### Fixed

- make `pattern` generation respect `minLength`/`maxLength`

### Other

- run rustfmt via the nightly toolchain
- fmt
- instruct agents to mark breaking changes in commit messages
- add AI-drafted schema corpus and integration harness

## [0.1.6](https://github.com/glennib/noson/compare/v0.1.5...v0.1.6) - 2026-08-07

### Added

- support the `pattern` keyword via regex string generation

### Other

- Merge pull request #26 from glennib/gb-pattern-keyword-generation

## [0.1.5](https://github.com/glennib/noson/compare/v0.1.4...v0.1.5) - 2026-08-07

### Fixed

- generate RFC 3339-valid durations for `format: duration`

### Other

- Merge pull request #22 from glennib/renovate/actions-checkout-7.x
- *(deps)* update dev-dependency jsonschema to 0.49.6
- *(deps)* update rust crate rand to v0.10.1
- *(deps)* update rust crate jsonschema to 0.45.0

## [0.1.4](https://github.com/glennib/noson/compare/v0.1.3...v0.1.4) - 2026-03-07

### Other

- *(deps)* update rust crate jsonschema to v0.44.1
- *(deps)* update rust crate jiff to v0.2.23
- Merge pull request #12 from glennib/renovate/jsonschema-0.x
- *(deps)* update rust crate jsonschema to 0.44.0

## [0.1.3](https://github.com/glennib/noson/compare/v0.1.2...v0.1.3) - 2026-02-18

### Added

- add `format: "date"`, `"time"`, and `"duration"` support for string generation
- add `format: "date-time"` support for string generation

### Other

- cargo fmt
- *(deps)* update actions/cache action to v5
- Add renovate.json

## [0.1.2](https://github.com/glennib/noson/compare/v0.1.1...v0.1.2) - 2026-02-16

### Fixed

- *(ci)* add contents:write permission to release-plz workflow

### Other

- add installation section and clarify generation behavior

## [0.1.1](https://github.com/glennib/noson/compare/v0.1.0...v0.1.1) - 2026-02-16

### Fixed

- validate min/max

### Other

- cargo fmt

## [0.1.0](https://github.com/glennib/noson/compare/v0.0.1...v0.1.0) - 2026-02-16

### Added

- initial crate implementation

### Other

- add release-plz workflow
