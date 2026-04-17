# Contributing to nsnfind

Thanks for your interest. This project follows a strict Git Flow and a
zero-warning Rust workflow — please read this before opening a PR.

## Quick start

```sh
git clone https://github.com/jchultarsky101/nsnfind
cd nsnfind
git config core.hooksPath .githooks     # optional pre-commit hook
./scripts/verify.sh                     # fmt-check + clippy + tests
```

## Branching (Git Flow)

| Branch type | Pattern              | Branched from | Merges into        |
|-------------|----------------------|---------------|--------------------|
| Feature     | `feature/<you>/<name>` | `develop`   | `develop` (`--no-ff`) |
| Release     | `release/<version>`  | `develop`     | `main` + `develop` (tag on `main`) |
| Hotfix      | `hotfix/<name>`      | `main`        | `main` + `develop` (tag on `main`) |

**Never commit directly to `main` or `develop`.** Every change — even a one-line fix — goes through a branch and a PR.

## Before you open a PR

- [ ] `./scripts/verify.sh` passes locally (fmt-check, clippy with `-D warnings`, tests).
- [ ] New behavior is covered by a test (unit test in-module or integration test under `tests/`).
- [ ] No `.unwrap()` / `panic!` in production code — use `Result` / `Option` with `?`.
- [ ] Errors in library modules use `thiserror`; CLI/app layer uses `anyhow`.
- [ ] User-facing changes are reflected in `CHANGELOG.md` under `[Unreleased]`.
- [ ] No new dependency unless it earns its place (check `Cargo.toml` first — prefer `std`).
- [ ] No secrets, tokens, or real credentials in tests, fixtures, or commit messages.

## Commit messages

Use conventional-ish prefixes when practical: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`, `ci:`. Keep the subject line under 72 chars; put the "why" in the body.

## Code style

- `cargo fmt` enforced; the default `rustfmt` settings apply.
- `cargo clippy --all-targets -- -D warnings` enforced — fix lints, don't `#[allow]` them unless you can justify it in a comment.
- Prefer explicit types at public API boundaries; let inference work in locals.
- Default to no comments — the code should be readable. A comment should explain *why*, never *what*.

## Adding a new backend

The long-term plan is multi-backend parts lookup. `IlsClient` and `soap.rs` are intentionally isolated under the ILS name; do not collapse new backends into them. Introduce a backend trait (or enum of concrete clients) at the CLI layer and keep each backend's types/errors local.

## Reporting security issues

Do **not** open a public issue for a credential leak or an auth-bypass concern. Email the maintainer directly.
