# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `government` subcommand — calls ILSmart `GetGovernmentData` for each NSN/NIIN with a configurable dataset list (default `MCRL`). JSON and CSV output. Added government-side SOAP request builder, response types (item name, FSC, NIIN, `HasPartsAvailability`, MCRL cross-reference, NSN info, procurement history), and a case-insensitive boolean parser for `HasPartsAvailability`.
- `lookup` subcommand — combined flow: calls `GetGovernmentData` first, then `GetPartsAvailability` only when the catalog indicates live listings. Outputs a merged per-NSN record; status codes distinguish `not_in_catalog`, `catalog_only`, `catalog_only_no_listings`, `catalog_only_avail_error`, and `ok`.
- Dataset enum with parse/serialize for the 12 `GovFilesToSearch` codes (AMDF, CRF, DLA, ISDOD, ISUSAF, MCRL, MLC, MOE, MRIL, NHA, PH, TECH).
- Subcommand aliases: `availability`/`parts`/`avail`, `government`/`gov`/`govdata`, `lookup`/`check`/`all`.

### Changed
- **Breaking:** `query` subcommand removed; replaced by `availability` with the same arguments. `QueryResult` is now generic over the operation's result type.
- SOAP client split into a module tree (`soap::availability`, `soap::government`) with shared envelope/fault helpers in `soap::mod`.

### Fixed
- Validator for `credentials.user_id` now accepts the `U01` suffix case-insensitively (observed: ILSmart normalizes to uppercase server-side).
- `--datasets` / `--gov-datasets` runtime panic: the custom value parser returned `Vec<Dataset>` but clap's derive expected a single `Dataset` per delimited value. Parser now returns one `Dataset`; comma splitting is delegated to clap via `value_delimiter = ','`.

### Verified
- End-to-end smoke test against ILSmart TEST MODE (`TESTU01` / `WEBSERVICETEST`, documented in SDK v5.0): `availability`, `government` (with default and `--datasets MCRL,PH`), and `lookup` all return populated responses and render JSON/CSV correctly.

## [0.1.0] — initial scaffold

### Added
- Initial `nsnfind` CLI with `query` and `config {path,show,set}` subcommands.
- ILSmart SOAP 1.1 `GetPartsAvailability` client (reqwest + quick-xml).
- NSN/NIIN input parser accepting formatted (`4730-01-234-5678`) and raw (`4730012345678`, 9-digit NIIN) forms.
- JSON and CSV output formatters with per-input status reporting.
- TOML config loader with `--config` / `$NSNFIND_CONFIG` / `./nsnfind.toml` / `~/.config/nsnfind/config.toml` precedence and redacted-password `Debug`.
- Bounded-concurrency query loop (default 4 in-flight, configurable).
- GitHub Actions CI (fmt + clippy + test + release build) and opt-in pre-commit hook.

[Unreleased]: https://github.com/jchultarsky101/nsnfind/commits/develop
