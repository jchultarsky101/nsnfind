# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial `nsnfind` CLI with `query` and `config {path,show,set}` subcommands.
- ILSmart SOAP 1.1 `GetPartsAvailability` client (reqwest + quick-xml).
- NSN/NIIN input parser accepting formatted (`4730-01-234-5678`) and raw (`4730012345678`, 9-digit NIIN) forms.
- JSON and CSV output formatters with per-input status reporting.
- TOML config loader with `--config` / `$NSNFIND_CONFIG` / `./nsnfind.toml` / `~/.config/nsnfind/config.toml` precedence and redacted-password `Debug`.
- Bounded-concurrency query loop (default 4 in-flight, configurable).
- GitHub Actions CI (fmt + clippy + test + release build) and opt-in pre-commit hook.

[Unreleased]: https://github.com/jchultarsky101/nsnfind/commits/develop
