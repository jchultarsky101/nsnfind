#!/usr/bin/env bash
# Run the full verification suite: fmt, clippy (warnings = errors), tests.
# CLAUDE.md expects this script to pass before any commit.
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

cd "$(dirname "$0")/.."

say() { printf '\n=== %s ===\n' "$1"; }

say "cargo fmt --check"
cargo fmt --all -- --check

say "cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

say "cargo test"
cargo test --all-features --no-fail-fast

say "All checks passed"
