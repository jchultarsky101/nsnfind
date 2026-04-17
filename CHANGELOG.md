# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `government` subcommand — queries the US DoD government catalog for each NSN/NIIN with a configurable dataset list (default `MCRL`). JSON and CSV output. Returns item name, FSC, NIIN, `HasPartsAvailability` gate, MCRL cross-reference, NSN info, and procurement history.
- `lookup` subcommand — combined flow: catalog lookup first, then supplier listings only when the catalog indicates live inventory. Outputs a merged per-NSN record; status codes distinguish `not_in_catalog`, `catalog_only`, `catalog_only_no_listings`, `catalog_only_avail_error`, and `ok`.
- Dataset enum with parse/serialize for the 12 US DoD dataset codes (AMDF, CRF, DLA, ISDOD, ISUSAF, MCRL, MLC, MOE, MRIL, NHA, PH, TECH).
- Subcommand aliases: `availability`/`parts`/`avail`, `government`/`gov`/`govdata`, `lookup`/`check`/`all`.

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

[Unreleased]: https://github.com/jchultarsky101/nsnfind/commits/develop
