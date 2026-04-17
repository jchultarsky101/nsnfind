# nsnfind

[![CI](https://github.com/jchultarsky101/nsnfind/actions/workflows/ci.yml/badge.svg)](https://github.com/jchultarsky101/nsnfind/actions/workflows/ci.yml)

Command-line tool that looks up parts availability by **NSN** (National Stock Number) or **NIIN** across one or more supplier backends. The first backend is [ILSmart](https://www.ilsmart.com) via its SOAP `GetPartsAvailability` service; the CLI is intentionally backend-neutral so more sources can be added later.

---

## Features

- Read a flat text file of NSNs / NIINs (one per line; `#` comments and blank lines are ignored).
- Validate and normalize input (accepts `4730-01-234-5678`, `4730 01 234 5678`, `4730012345678`, or 9-digit NIIN).
- Call ILSmart's SOAP endpoint once per part number with bounded concurrency (default 4 in-flight).
- Emit **JSON** (default, pretty) or **CSV** (flat, one row per supplier match).
- Credentials stored in a local TOML file (never committed) or set via CLI subcommand.
- SOAP faults, HTTP errors, and invalid inputs are reported per-row — one bad NSN never aborts the run.

## Install

```sh
git clone https://github.com/jchultarsky101/nsnfind
cd nsnfind
cargo install --path .
```

The binary is installed as `nsnfind` into `~/.cargo/bin`.

## Configure

Three equivalent options:

**1. Config file template (recommended for first-time setup).**
```sh
cp nsnfind.toml.example nsnfind.toml   # gitignored
$EDITOR nsnfind.toml                   # fill in user_id + password
```

**2. User-scope config (out of repo).**
```sh
mkdir -p ~/.config/nsnfind
cp nsnfind.toml.example ~/.config/nsnfind/config.toml
```

**3. CLI (no shell-history leak for the password).**
```sh
nsnfind config set --user-id ABCDEFU01 --password-stdin <<<'yourPassword'
```
Writes `~/.config/nsnfind/config.toml` with mode `0600`.

Resolution order (first match wins):

1. `--config <path>`
2. `$NSNFIND_CONFIG`
3. `./nsnfind.toml`
4. `~/.config/nsnfind/config.toml`

ILSmart credential rules enforced at load time:
- `user_id`: up to 10 alphanumeric ASCII chars, must end in `U01`.
- `password`: 6–20 chars (case-sensitive).

## Usage

```sh
nsnfind query docs/sample-nsns.txt                # JSON to stdout
nsnfind query docs/sample-nsns.txt --format csv   # CSV to stdout
nsnfind query docs/sample-nsns.txt -o out.json    # write to file
nsnfind query docs/sample-nsns.txt -v             # INFO logs to stderr
nsnfind config path                               # resolved config file
nsnfind config show                               # print effective config (password redacted)
```

Set `RUST_LOG=nsnfind=debug` for more detail; `-v`/`-vv`/`-vvv` are shortcuts.

### Output shape

**JSON** (per input line):
```json
[
  {
    "line": 1,
    "input": "3020-01-204-7592",
    "normalized": "3020012047592",
    "status": "ok",
    "listings": [
      {
        "company": { "id": "...", "name": "...", "supplier_cage": "..." },
        "parts": { "items": [ { "part_number": "...", "condition_code": "NE", "quantity": "1", "..." : "..." } ] }
      }
    ]
  }
]
```

`status` ∈ `ok | no_results | api_fault | error | invalid`.

**CSV** is a 22-column flat table, one row per (input, supplier, matching part). Inputs with no match, invalid NSNs, and transport errors each get a single summary row.

## Development

```sh
./scripts/verify.sh              # fmt --check + clippy -D warnings + tests
cargo test                       # unit tests only
cargo fmt                        # auto-format
cargo clippy --all-targets -- -D warnings
```

Optional pre-commit hook:
```sh
git config core.hooksPath .githooks
```
Runs `verify.sh` before each commit touching Rust sources or Cargo files. Override with `--no-verify` only if strictly necessary — fix the issue instead.

### Branching

We use **Git Flow**:

| Branch type | Pattern              | Branched from | Merges into        |
|-------------|----------------------|---------------|--------------------|
| Feature     | `feature/jc/<name>`  | `develop`     | `develop` (`--no-ff`) |
| Release     | `release/<version>`  | `develop`     | `main` + `develop` (tag on `main`) |
| Hotfix      | `hotfix/<name>`      | `main`        | `main` + `develop` (tag on `main`) |

**No direct commits to `main` or `develop`** — always branch.

## Project layout

```
src/
  cli.rs       clap definitions + query/config subcommand handlers
  client.rs    reqwest-backed ILSmart HTTP client, bounded concurrency
  config.rs    TOML config loader with precedence + validation
  error.rs     thiserror enum (IlsError)
  nsn.rs       NSN/NIIN parser and validator
  output.rs    JSON + CSV formatters
  soap.rs      SOAP envelope builder + response parser (quick-xml + serde)
docs/
  ILS Web Services SDK v4.1.pdf        vendor API reference
  ILSmart - sample NSN codes.png       test fixture for docs/sample-nsns.txt
  sample-nsns.txt                      5 real NSNs for smoke testing
```

## License

Dual-licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
