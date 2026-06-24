# Changelog

All notable changes to the Rust license client are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-06-24

Initial release.

### Added
- `LicenseClient` with `activate()`, `deactivate()`, `validate()`, `is_valid()`,
  `swap()`, `activations()` and `check_for_update()`.
- Blocking HTTP via `ureq`; JSON via `serde_json`.
- `instance` defaults to the machine hostname when not provided.
- Transport failures surface as `LicenseResult { ok: false, status: 0, body: {"error":"network_error"} }`
  rather than a panic.

[Unreleased]: https://github.com/licensetorun/rust-license-client/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/licensetorun/rust-license-client/releases/tag/v1.0.0
