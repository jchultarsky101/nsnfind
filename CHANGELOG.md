# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] — 2026-04-29

### Removed
- `government` subcommand and the underlying `soap::government` SOAP module — the upstream government-data endpoint does not return useful results.
- `lookup` (combined) subcommand that chained government + availability calls.

### Changed
- **Breaking:** `availability` subcommand renamed to `lookup` (short alias `ls`). The old names `availability` and `avail` are kept as aliases for backward compatibility.
- CLI description updated to reflect the single-operation focus.

## [0.2.3] — 2026-04-28

### Changed
- cargo-dist bumped 0.29.0 → 0.31.0. Regenerated release workflow uses `actions/checkout@v6`, `actions/upload-artifact@v6`, and `actions/download-artifact@v7`, eliminating the Node.js 20 deprecation warnings GitHub will start enforcing in June 2026.

## [0.2.2] — 2026-04-28

### Fixed
- Release pipeline pinned to `macos-latest` for both Apple targets. cargo-dist 0.29 defaults both `aarch64-apple-darwin` and `x86_64-apple-darwin` to `macos-13`, which is in GitHub's deprecation window and now hangs in the queue indefinitely. `macos-latest` (currently macos-14, Apple Silicon) builds aarch64 natively and cross-compiles x86_64 via the bundled Rust toolchain. v0.2.1's release workflow was canceled mid-run for this reason; v0.2.2 is the first release that publishes binaries successfully.

## [0.2.1] — 2026-04-28

### Added
- cargo-dist release pipeline: tag pushes (`v*`) build Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64) binaries; produce a Windows MSI; ship shell and PowerShell installers and the cargo-dist updater. Manifest fields (`authors`, `homepage`, `documentation`, `readme`, `keywords`, `categories`) added to satisfy installer metadata. WiX upgrade/path GUIDs locked in `[package.metadata.wix]` so MSI upgrades stay coherent across releases.

### Note
- v0.2.0 was tagged before this pipeline existed and ships without binary artifacts. v0.2.1 is the first release with attached binaries and installers.

## [0.2.0] — 2026-04-28

### Added
- `government` subcommand — queries the US DoD government catalog for each NSN/NIIN with a configurable dataset list (default `MCRL`). JSON and CSV output. Returns item name, FSC, NIIN, `HasPartsAvailability` gate, MCRL cross-reference, NSN info, and procurement history.
- `lookup` subcommand — combined flow: catalog lookup first, then supplier listings only when the catalog indicates live inventory. Outputs a merged per-NSN record; status codes distinguish `not_in_catalog`, `catalog_only`, `catalog_only_no_listings`, `catalog_only_avail_error`, and `ok`.
- Dataset enum with parse/serialize for the 12 US DoD dataset codes (AMDF, CRF, DLA, ISDOD, ISUSAF, MCRL, MLC, MOE, MRIL, NHA, PH, TECH).
- Subcommand aliases: `availability`/`parts`/`avail`, `government`/`gov`/`govdata`, `lookup`/`check`/`all`.
- `docs/single-nsn.txt` smoke-test fixture (one NSN, for fast manual checks).

### Changed
- **Breaking:** `query` subcommand removed; replaced by `availability` with the same arguments. Results are now generic over the per-operation payload type.
- SOAP client split into a module tree (`soap::availability`, `soap::government`) with shared envelope/fault helpers.
- Documentation rewritten to be backend-neutral; the underlying upstream service is described only in one "Backend" section.

### Fixed
- Validator for `credentials.user_id` accepts the `U01` suffix case-insensitively (upstream normalizes to uppercase server-side).
- `--datasets` / `--gov-datasets` runtime panic: the custom value parser returned `Vec<Dataset>` but clap's derive expected a single `Dataset` per delimited value. Parser now returns one `Dataset`; comma splitting is delegated to clap via `value_delimiter = ','`.

### Verified
- End-to-end smoke test against the upstream's documented TEST MODE account: `availability`, `government` (with default and `--datasets MCRL,PH`), and `lookup` all return populated responses and render JSON/CSV correctly.

## [0.1.0] — initial scaffold

### Added
- Initial `nsnfind` CLI with a single query subcommand and `config {path,show,set}` subcommands.
- SOAP 1.1 client for the parts-availability upstream (reqwest + quick-xml).
- NSN/NIIN input parser accepting formatted (`4730-01-234-5678`) and raw (`4730012345678`, 9-digit NIIN) forms.
- JSON and CSV output formatters with per-input status reporting.
- TOML config loader with `--config` / `$NSNFIND_CONFIG` / `./nsnfind.toml` / `~/.config/nsnfind/config.toml` precedence and redacted-password `Debug`.
- Bounded-concurrency query loop (default 4 in-flight, configurable).
- GitHub Actions CI (fmt + clippy + test + release build) and opt-in pre-commit hook.

[Unreleased]: https://github.com/jchultarsky101/nsnfind/compare/v0.3.0...develop
[0.3.0]: https://github.com/jchultarsky101/nsnfind/compare/v0.2.3...v0.3.0
[0.2.3]: https://github.com/jchultarsky101/nsnfind/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/jchultarsky101/nsnfind/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/jchultarsky101/nsnfind/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/jchultarsky101/nsnfind/compare/v0.1.0...v0.2.0
