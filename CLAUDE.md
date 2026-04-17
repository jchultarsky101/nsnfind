# Guardrails — Ember Trove

Act as a Senior Rust Architect. We follow a **zero-panic, TDD-first** workflow.
Before finalising any file edit, run `cargo check` and `cargo clippy`.
Output only complete, idiomatic Rust. Use `thiserror` for all custom error types.

**Self-learning resources** (grep before debugging or writing new patterns):
- `.claude/ERRORS.md` — known compile/runtime error patterns and fixes
- `.claude/patterns/` — canonical code patterns (navigate, submit, debounce, double-opt)
- `.claude/rules/leptos.md` — Leptos-specific rules (auto-loaded for ui/ files)
- `.claude/rules/api.md` — API/backend rules (auto-loaded for api/ files)
- `.claude/ROADMAP.md` — current state, backlog, architecture decisions

---

## Performance & Personality

- **Dense Mode**: Minimal conversational fluff; focus on production-ready code.
- **No placeholders** (`// ...`): All code must be complete and compilable.

## Safety & Idioms

- **No Panics**: Never use `.unwrap()` or `panic!`. Use `Result`/`Option` with `?`.
- **Error Handling**: `thiserror` for library errors, `anyhow` for application-level.
- **Ownership**: Follow borrow-checker rules. Prefer owned types initially.
- **Dependencies**: Check `Cargo.toml` before adding crates. Prefer `std`.

## Development Workflow (TDD)

1. **Red** — Write a failing test in `tests/` or a `mod tests` block.
2. **Green** — Implement the minimal logic to pass the test.
3. **Refactor** — `cargo clippy -- -D warnings` + `cargo fmt`.

## Post-Edit Commands

After any `.rs` edit in `api/` or `common/`:
```
cargo check && cargo clippy -- -D warnings
```

After any `.rs` edit in `ui/` (WASM):
```
cargo check -p ui --target wasm32-unknown-unknown
cargo clippy -p ui --target wasm32-unknown-unknown -- -D warnings
```

Before any `git commit`:
```
cargo test
```

Run `./scripts/verify.sh` for a full suite (all of the above + git status check).

---

## Git Flow

| Branch type | Pattern              | Branched from | Merges into        | Notes                           |
|-------------|----------------------|---------------|--------------------|---------------------------------|
| Feature     | `feature/jc/<name>`  | `develop`     | `develop`          | `--no-ff`; worktree per feature |
| Release     | `release/<version>`  | `develop`     | `main` + `develop` | tag on `main` after merge       |
| Hotfix      | `hotfix/<name>`      | `main`        | `main` + `develop` | tag patch bump on `main`        |

- **Never commit directly to `main` or `develop`** — all changes via branches.
- Use `/release [major|minor|patch]` command for release workflow.
- Use `./scripts/next-version.sh` to compute the next semver automatically.

## Environment Quirks

- **`cargo` PATH**: `export PATH="$HOME/.cargo/bin:$PATH"` in every Bash tool call.
- **`cat` aliased to `bat`**: Use plain `-m "..."` for git commit messages (not heredoc).
- **`grep`/`tail`/`head`/`rg` not available**: Use Grep tool; Read with offset/limit; `python3 -c` for JSON.
- **`aws` CLI unavailable**: Use `boto3` via `pip3 install boto3` + Python.
- **Worktree cwd resets**: Always use absolute paths in Bash tool calls.
- **Worktree dir deleted → shell broken**: `Write` a `.keep` file at `<path>/.keep` to fix, then `git worktree prune`.
