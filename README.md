# nsnfind

[![CI](https://github.com/jchultarsky101/nsnfind/actions/workflows/ci.yml/badge.svg)](https://github.com/jchultarsky101/nsnfind/actions/workflows/ci.yml)

Command-line tool that looks up information about military parts by **NSN** (National Stock Number) or **NIIN** (National Item Identification Number). Three modes:

- **`availability`** — who is currently selling this part
- **`government`** — what the US government catalog says about it (item name, FSC, CAGE cross-reference, procurement history)
- **`lookup`** — the combined view: run the government lookup first, then fall through to supplier listings when the catalog indicates live inventory exists

The CLI surface is intentionally backend-agnostic; the underlying data source is configurable and may change over time without breaking your scripts.

---

## Features

- Read a flat text file of NSNs / NIINs (one per line; `#` comments and blank lines are ignored).
- Validate and normalize input (accepts `4730-01-234-5678`, `4730 01 234 5678`, `4730012345678`, or 9-digit NIIN).
- Bounded concurrency (default 4 in-flight) and per-request timeouts.
- Emit **JSON** (default, pretty) or **CSV** (flat, one row per result) — the JSON shape is stable and ideal for piping into `jq`; CSV is ideal for spreadsheets.
- Credentials stored in a local TOML file (never committed) or set via CLI subcommand. Password-stdin flag to keep it out of shell history.
- Per-row status codes (`ok`, `no_results`, `catalog_only`, `not_in_catalog`, `error`, `invalid`, etc.) — one bad input never aborts the run.

## Install

```sh
git clone https://github.com/jchultarsky101/nsnfind
cd nsnfind
cargo install --path .
```

The binary is installed as `nsnfind` into `~/.cargo/bin`.

## Quick smoke test

The upstream service publishes a shared **TEST MODE** account that returns static sample data, useful for verifying connectivity and output shape before your real credentials are entitled. See the vendor SDK under `docs/` for the current test user/password, or contact your account representative. Drop them into a throwaway config and run:

```sh
nsnfind --config /tmp/nsnfind-test.toml availability docs/sample-nsns.txt
nsnfind --config /tmp/nsnfind-test.toml government docs/sample-nsns.txt
nsnfind --config /tmp/nsnfind-test.toml lookup docs/sample-nsns.txt
```

**Don't build business logic against TEST MODE data** — the values are fixtures, not representative. Use it only for wire-format and pipeline checks.

## Configure

Three equivalent options for real credentials:

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

**3. CLI, no shell-history leak.**
```sh
nsnfind config set --user-id YOURIDU01 --password-stdin <<<'yourPassword'
```
Writes `~/.config/nsnfind/config.toml` with mode `0600`.

Resolution order (first match wins):

1. `--config <path>`
2. `$NSNFIND_CONFIG`
3. `./nsnfind.toml`
4. `~/.config/nsnfind/config.toml`

Credential constraints enforced at load time:
- `user_id`: up to 10 alphanumeric ASCII chars, must end in `U01` (case-insensitive).
- `password`: 6–20 chars (case-sensitive).

## Usage

Three query subcommands, each reads the same newline-delimited NSN/NIIN input file:

```sh
# Who is currently selling this part?
nsnfind availability docs/sample-nsns.txt

# What does the government catalog say about it?
nsnfind government docs/sample-nsns.txt
nsnfind government docs/sample-nsns.txt --datasets MCRL,PH

# Combined: catalog lookup first, then suppliers when the catalog flags live listings.
nsnfind lookup docs/sample-nsns.txt
nsnfind lookup docs/sample-nsns.txt --gov-datasets MCRL,PH

# Common flags
nsnfind <subcommand> <input> --format csv     # default is json
nsnfind <subcommand> <input> -o out.json      # write to file
nsnfind <subcommand> <input> -v               # INFO logs to stderr
```

Aliases: `availability`|`parts`|`avail`, `government`|`gov`|`govdata`, `lookup`|`check`|`all`.

Config management:

```sh
nsnfind config path                           # resolved config file
nsnfind config show                           # print effective config (password redacted)
nsnfind config set --user-id X --password-stdin <<<'yourPassword'
```

Set `RUST_LOG=nsnfind=debug` for more detail; `-v`/`-vv`/`-vvv` are shortcuts.

### Government dataset codes

`--datasets` / `--gov-datasets` accept a comma-separated list of US DoD dataset codes defined by the Defense Logistics Agency in DOD 4100.39-M (FLIS Procedures Manual, Volume 10):

| Code     | Meaning                                                           |
|----------|-------------------------------------------------------------------|
| `AMDF`   | Army Master Data File                                             |
| `CRF`    | Navy Cross Reference File                                         |
| `DLA`    | DLA Forecast Data                                                 |
| `ISDOD`  | DoD Interchangeability & Substitutability                         |
| `ISUSAF` | Air Force Interchangeability & Substitutability                   |
| `MCRL`   | Master Cross Reference List (manufacturer → part number → CAGE)   |
| `MLC`    | Management List – Consolidated                                    |
| `MOE`    | Major Organizational Entity Rule                                  |
| `MRIL`   | Navy Master Repairable Item List                                  |
| `NHA`    | Navy Next Higher Assembly                                         |
| `PH`     | Procurement History                                               |
| `TECH`   | Technical Characteristics                                         |

Default is `MCRL`, which gives you the item name, FSC, NIIN, the
`HasPartsAvailability` gate used by `lookup`, and manufacturer cross-reference rows.

### Output shape

**JSON** — one record per input line. Each record carries a `status` field:

| subcommand     | status values                                                                                          |
|----------------|--------------------------------------------------------------------------------------------------------|
| `availability` | `ok`, `no_results`, `api_fault`, `error`, `invalid`                                                    |
| `government`   | same, plus surfaces `faults[]` / `items[]`                                                             |
| `lookup`       | `ok`, `catalog_only`, `catalog_only_no_listings`, `catalog_only_avail_error`, `not_in_catalog`, `gov_fault`, `error`, `invalid` |

**CSV** — flat per-subcommand tables:

- `availability` → 22 columns; one row per (input, supplier, matching part).
- `government` → 12 columns; one row per (input, MCRL cross-reference item).
- `lookup` → 26 columns; merges catalog metadata (`item_name`, `fsc`, `niin`, `has_parts_availability`, primary MCRL CAGE/company) with supplier details from `availability`.

Inputs with no match, invalid NSNs, and transport errors each get a single summary row.

## Development

```sh
./scripts/verify.sh                        # fmt --check + clippy -D warnings + tests
cargo test                                 # unit tests only
cargo fmt                                  # auto-format
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
  cli.rs              clap definitions + query/config subcommand handlers
  client.rs           HTTP client (reqwest) with bounded concurrency
  config.rs           TOML config loader with precedence + validation
  error.rs            thiserror enum
  nsn.rs              NSN/NIIN parser and validator
  output.rs           JSON + CSV formatters per subcommand
  soap/               SOAP wire-format support
    mod.rs            shared envelope / fault / xml-escape helpers
    availability.rs   request builder + response types for the availability op
    government.rs     request builder + response types for the government op
docs/
  sample-nsns.txt     5 real NSNs for smoke testing
```

## Backend

The CLI is currently backed by a third-party SOAP-based parts-availability service. The internal module names (`IlsClient`, `soap::availability`, `soap::government`) reflect the current provider, but the user-facing CLI surface — subcommand names, flags, output shape — is intentionally provider-neutral so the backend can be swapped or supplemented without breaking callers.

Vendor SDK references are kept under `docs/` for developers.

## License

Dual-licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
