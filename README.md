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

Three query subcommands, each reads the same newline-delimited NSN/NIIN input file:

```sh
# Who is currently selling this NSN? (ILSmart GetPartsAvailability, WS Buyer Bundle)
nsnfind availability docs/sample-nsns.txt

# What does the government catalog say about this NSN? (ILSmart GetGovernmentData, ILS Gov Options)
nsnfind government docs/sample-nsns.txt
nsnfind government docs/sample-nsns.txt --datasets MCRL,PH

# Combined: call government first, then availability if the catalog flags HasPartsAvailability=true.
nsnfind lookup docs/sample-nsns.txt
nsnfind lookup docs/sample-nsns.txt --gov-datasets MCRL,PH

# Common flags
nsnfind <subcommand> <input> --format csv     # default is json
nsnfind <subcommand> <input> -o out.json      # write to file
nsnfind <subcommand> <input> -v               # INFO logs to stderr
```

Aliases are registered: `availability`|`parts`|`avail`, `government`|`gov`|`govdata`, `lookup`|`check`|`all`.

Config management is unchanged:

```sh
nsnfind config path                           # resolved config file
nsnfind config show                           # print effective config (password redacted)
nsnfind config set --user-id X --password-stdin <<<'yourPassword'
```

Set `RUST_LOG=nsnfind=debug` for more detail; `-v`/`-vv`/`-vvv` are shortcuts.

### Output shape

**JSON** — one record per input line, shape depends on subcommand. Each record carries a `status` field:

| subcommand     | status values                                                                                          |
|----------------|--------------------------------------------------------------------------------------------------------|
| `availability` | `ok`, `no_results`, `api_fault`, `error`, `invalid`                                                   |
| `government`   | same plus surfaces `faults[]` / `items[]`                                                              |
| `lookup`       | `ok`, `catalog_only`, `catalog_only_no_listings`, `catalog_only_avail_error`, `not_in_catalog`, `gov_fault`, `error`, `invalid` |

**CSV** — flat per-subcommand tables:

- `availability` → 22 columns; one row per (input, supplier, matching part).
- `government` → 12 columns; one row per (input, MCRL cross-reference item).
- `lookup` → 26 columns; merges government metadata (`item_name`, `fsc`, `niin`, `has_parts_availability`, primary MCRL CAGE/company) with supplier details from `availability`.

Inputs with no match, invalid NSNs, and transport errors each get a single summary row.

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
