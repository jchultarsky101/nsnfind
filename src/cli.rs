use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::client::IlsClient;
use crate::config::{
    Config, DEFAULT_CONCURRENCY, DEFAULT_ENDPOINT, DEFAULT_TIMEOUT_SECS, ends_with_u01,
};
use crate::nsn::{InputEntry, parse_nsn_list};
use crate::output;
use crate::soap::government::Dataset;

#[derive(Debug, Parser)]
#[command(
    name = "nsnfind",
    version,
    about = "Query parts-availability backends (ILSmart SOAP today) by NSN/NIIN"
)]
pub struct Args {
    /// Path to the config file (TOML). Overrides $NSNFIND_CONFIG, ./nsnfind.toml,
    /// and $HOME/.config/nsnfind/config.toml in that order.
    #[arg(short = 'c', long, value_name = "PATH", global = true)]
    pub config: Option<PathBuf>,

    /// Increase verbosity (-v info, -vv debug, -vvv trace). RUST_LOG overrides.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Who is currently selling this NSN/NIIN? (ILSmart GetPartsAvailability)
    #[command(alias = "parts", alias = "avail")]
    Availability(CommonQueryArgs),

    /// What does the US government catalog say about this NSN/NIIN?
    /// (ILSmart GetGovernmentData)
    #[command(alias = "gov", alias = "govdata")]
    Government(GovernmentArgs),

    /// Combined lookup: government catalog first, then marketplace suppliers
    /// when the catalog indicates there are live listings.
    #[command(alias = "check", alias = "all")]
    Lookup(LookupArgs),

    /// Manage the config file
    #[command(subcommand)]
    Config(ConfigCommand),
}

#[derive(Debug, clap::Args)]
pub struct CommonQueryArgs {
    /// Path to a flat text file with one NSN or NIIN per line.
    /// Blank lines and lines beginning with '#' are ignored.
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = Format::Json)]
    pub format: Format,

    /// Write output to this file instead of stdout.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct GovernmentArgs {
    #[command(flatten)]
    pub common: CommonQueryArgs,

    /// Comma-separated government datasets to query.
    /// Valid values: AMDF, CRF, DLA, ISDOD, ISUSAF, MCRL, MLC, MOE, MRIL, NHA, PH, TECH.
    #[arg(
        long,
        value_name = "LIST",
        default_value = "MCRL",
        value_delimiter = ',',
        value_parser = parse_dataset,
    )]
    pub datasets: Vec<Dataset>,
}

#[derive(Debug, clap::Args)]
pub struct LookupArgs {
    #[command(flatten)]
    pub common: CommonQueryArgs,

    /// Government datasets to query in the first call. Default: MCRL (enough to
    /// retrieve ItemName, FSC, CAGE cross-reference, and the
    /// HasPartsAvailability flag that gates the second call).
    #[arg(
        long,
        value_name = "LIST",
        default_value = "MCRL",
        value_delimiter = ',',
        value_parser = parse_dataset,
    )]
    pub gov_datasets: Vec<Dataset>,
}

fn parse_dataset(s: &str) -> Result<Dataset, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty dataset value".to_owned());
    }
    Dataset::parse(trimmed).ok_or_else(|| format!("unknown dataset {trimmed:?}"))
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the resolved config file path
    Path,

    /// Print the effective config (password is redacted)
    Show,

    /// Set config values. Creates the file if it doesn't exist.
    Set(ConfigSetArgs),
}

#[derive(Debug, clap::Args)]
pub struct ConfigSetArgs {
    /// ILSmart UserId (up to 10 alphanumeric chars, ending in U01)
    #[arg(long)]
    pub user_id: Option<String>,

    /// ILSmart password (6-20 chars). Leaks into shell history; prefer --password-stdin.
    #[arg(long, conflicts_with = "password_stdin")]
    pub password: Option<String>,

    /// Read the password from stdin (single line)
    #[arg(long)]
    pub password_stdin: bool,

    /// Override the service endpoint URL
    #[arg(long)]
    pub endpoint: Option<String>,

    /// Max concurrent in-flight requests (>= 1)
    #[arg(long)]
    pub concurrency: Option<usize>,

    /// Per-request timeout in seconds (>= 1)
    #[arg(long)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "lower")]
pub enum Format {
    Json,
    Csv,
}

pub async fn run() -> Result<()> {
    let args = Args::parse();
    init_tracing(args.verbose);

    match args.command {
        Command::Availability(q) => run_availability(args.config.as_deref(), q).await,
        Command::Government(g) => run_government(args.config.as_deref(), g).await,
        Command::Lookup(l) => run_lookup(args.config.as_deref(), l).await,
        Command::Config(ConfigCommand::Path) => run_config_path(args.config.as_deref()),
        Command::Config(ConfigCommand::Show) => run_config_show(args.config.as_deref()),
        Command::Config(ConfigCommand::Set(s)) => run_config_set(args.config.as_deref(), s),
    }
}

async fn run_availability(config_path: Option<&Path>, args: CommonQueryArgs) -> Result<()> {
    let (config, entries) = prepare(config_path, &args.input)?;
    let client = IlsClient::new(&config).context("failed to build HTTP client")?;
    let results = client.run_availability(entries).await;
    emit(&args, |w| {
        output::write_availability(args.format, &results, w)
    })
}

async fn run_government(config_path: Option<&Path>, args: GovernmentArgs) -> Result<()> {
    let (config, entries) = prepare(config_path, &args.common.input)?;
    let client = IlsClient::new(&config).context("failed to build HTTP client")?;
    let results = client.run_government(entries, args.datasets.clone()).await;
    emit(&args.common, |w| {
        output::write_government(args.common.format, &results, w)
    })
}

async fn run_lookup(config_path: Option<&Path>, args: LookupArgs) -> Result<()> {
    let (config, entries) = prepare(config_path, &args.common.input)?;
    let client = IlsClient::new(&config).context("failed to build HTTP client")?;
    let results = client.run_lookup(entries, args.gov_datasets.clone()).await;
    emit(&args.common, |w| {
        output::write_combined(args.common.format, &results, w)
    })
}

fn prepare(config_path: Option<&Path>, input: &Path) -> Result<(Config, Vec<InputEntry>)> {
    let config = Config::load(config_path).context("failed to load config")?;
    if let Some(src) = &config.source {
        info!(
            config = %src.display(),
            endpoint = %config.api.endpoint,
            concurrency = config.api.concurrency,
            "configuration loaded"
        );
    }
    let input_text = std::fs::read_to_string(input)
        .with_context(|| format!("failed to read input {}", input.display()))?;
    let entries = parse_nsn_list(&input_text);
    let (valid, invalid) = entries
        .iter()
        .fold((0usize, 0usize), |(v, i), e| match e.parsed {
            Ok(_) => (v + 1, i),
            Err(_) => (v, i + 1),
        });
    info!(total = entries.len(), valid, invalid, "parsed input file");
    if entries.is_empty() {
        return Err(anyhow!(
            "input file contains no NSNs (after stripping blanks and comments)"
        ));
    }
    Ok((config, entries))
}

fn emit<F>(args: &CommonQueryArgs, writer: F) -> Result<()>
where
    F: FnOnce(Box<dyn Write>) -> anyhow::Result<()>,
{
    match args.output.as_deref() {
        Some(path) => {
            let file = std::fs::File::create(path)
                .with_context(|| format!("failed to create output {}", path.display()))?;
            writer(Box::new(std::io::BufWriter::new(file)))?;
            info!(path = %path.display(), "wrote output");
        }
        None => {
            let stdout = std::io::stdout().lock();
            writer(Box::new(stdout))?;
        }
    }
    Ok(())
}

fn run_config_path(explicit: Option<&Path>) -> Result<()> {
    let path = Config::resolve_existing(explicit)
        .or_else(default_user_config_path)
        .ok_or_else(|| anyhow!("cannot determine config path; set $HOME or pass --config"))?;
    println!("{}", path.display());
    Ok(())
}

fn run_config_show(explicit: Option<&Path>) -> Result<()> {
    let cfg = Config::load(explicit).context("failed to load config")?;
    let path = cfg
        .source
        .as_deref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<memory>".to_owned());
    println!("# config: {path}");
    println!("[credentials]");
    println!("user_id = {:?}", cfg.credentials.user_id);
    println!("password = \"<redacted>\"");
    println!();
    println!("[api]");
    println!("endpoint = {:?}", cfg.api.endpoint);
    println!("concurrency = {}", cfg.api.concurrency);
    println!("timeout_secs = {}", cfg.api.timeout_secs);
    Ok(())
}

fn run_config_set(explicit: Option<&Path>, set: ConfigSetArgs) -> Result<()> {
    let target = resolve_write_target(explicit)?;
    let mut doc = load_or_empty_document(&target)?;

    if let Some(uid) = &set.user_id {
        doc.set_credentials_user_id(uid.clone());
    }
    let password = if set.password_stdin {
        Some(read_password_stdin()?)
    } else {
        set.password.clone()
    };
    if let Some(pw) = &password {
        doc.set_credentials_password(pw.clone());
    }
    if let Some(ep) = &set.endpoint {
        doc.set_api_endpoint(ep.clone());
    }
    if let Some(c) = set.concurrency {
        doc.set_api_concurrency(c);
    }
    if let Some(t) = set.timeout_secs {
        doc.set_api_timeout_secs(t);
    }

    doc.validate()
        .with_context(|| format!("refusing to write invalid config to {}", target.display()))?;

    if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent dir {}", parent.display()))?;
    }
    let serialized = doc.to_toml_string();
    write_private(&target, serialized.as_bytes())
        .with_context(|| format!("failed to write {}", target.display()))?;
    println!("wrote {}", target.display());
    Ok(())
}

fn resolve_write_target(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p.to_owned());
    }
    if let Ok(p) = std::env::var("NSNFIND_CONFIG")
        && !p.is_empty()
    {
        return Ok(PathBuf::from(p));
    }
    let cwd = PathBuf::from("nsnfind.toml");
    if cwd.is_file() {
        return Ok(cwd);
    }
    default_user_config_path()
        .ok_or_else(|| anyhow!("cannot determine config path; set $HOME or pass --config"))
}

fn default_user_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/nsnfind/config.toml"))
}

fn read_password_stdin() -> Result<String> {
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .context("failed to read password from stdin")?;
    let pw = line.trim_end_matches(['\n', '\r']).to_owned();
    if pw.is_empty() {
        return Err(anyhow!("empty password on stdin"));
    }
    Ok(pw)
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

fn init_tracing(verbose: u8) {
    let default = match verbose {
        0 => "nsnfind=warn",
        1 => "nsnfind=info",
        2 => "nsnfind=debug",
        _ => "nsnfind=trace",
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

/// Minimal TOML editor that preserves round-trip semantics through `toml::Value`.
/// Comments are NOT preserved — users who want comments should edit the file directly.
struct ConfigDoc {
    root: toml::Table,
}

fn load_or_empty_document(path: &Path) -> Result<ConfigDoc> {
    let root = if path.is_file() {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str::<toml::Table>(&text)
            .with_context(|| format!("invalid TOML in {}", path.display()))?
    } else {
        toml::Table::new()
    };
    Ok(ConfigDoc { root })
}

impl ConfigDoc {
    fn credentials(&mut self) -> &mut toml::Table {
        ensure_table(&mut self.root, "credentials")
    }

    fn api(&mut self) -> &mut toml::Table {
        ensure_table(&mut self.root, "api")
    }

    fn set_credentials_user_id(&mut self, v: String) {
        self.credentials()
            .insert("user_id".to_owned(), toml::Value::String(v));
    }

    fn set_credentials_password(&mut self, v: String) {
        self.credentials()
            .insert("password".to_owned(), toml::Value::String(v));
    }

    fn set_api_endpoint(&mut self, v: String) {
        self.api()
            .insert("endpoint".to_owned(), toml::Value::String(v));
    }

    fn set_api_concurrency(&mut self, v: usize) {
        self.api()
            .insert("concurrency".to_owned(), toml::Value::Integer(v as i64));
    }

    fn set_api_timeout_secs(&mut self, v: u64) {
        self.api()
            .insert("timeout_secs".to_owned(), toml::Value::Integer(v as i64));
    }

    fn validate(&self) -> Result<()> {
        let creds = self
            .root
            .get("credentials")
            .and_then(|v| v.as_table())
            .ok_or_else(|| anyhow!("[credentials] section is required"))?;
        let uid = creds
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("credentials.user_id is required"))?;
        if !ends_with_u01(uid) {
            return Err(anyhow!(
                "credentials.user_id must end with 'U01' (case-insensitive); got {uid:?}"
            ));
        }
        if uid.len() > 10 || !uid.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(anyhow!(
                "credentials.user_id must be <= 10 alphanumeric ASCII chars"
            ));
        }
        let pw = creds
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("credentials.password is required"))?;
        let pw_len = pw.chars().count();
        if !(6..=20).contains(&pw_len) {
            return Err(anyhow!(
                "credentials.password must be between 6 and 20 characters"
            ));
        }
        if let Some(api) = self.root.get("api").and_then(|v| v.as_table()) {
            if let Some(c) = api.get("concurrency").and_then(|v| v.as_integer())
                && c < 1
            {
                return Err(anyhow!("api.concurrency must be >= 1"));
            }
            if let Some(t) = api.get("timeout_secs").and_then(|v| v.as_integer())
                && t < 1
            {
                return Err(anyhow!("api.timeout_secs must be >= 1"));
            }
        }
        Ok(())
    }

    fn to_toml_string(&self) -> String {
        let mut out = String::new();
        out.push_str("# ILSmart CLI configuration\n");
        out.push_str("# Generated by `ils config set`. Free-form edits are preserved on load\n");
        out.push_str("# but will be rewritten (without comments) on the next `config set`.\n");
        out.push('\n');
        if let Some(creds) = self.root.get("credentials") {
            out.push_str("[credentials]\n");
            append_table(&mut out, creds.as_table().unwrap_or(&toml::Table::new()));
            out.push('\n');
        }
        let default_endpoint = toml::Value::String(DEFAULT_ENDPOINT.to_owned());
        let default_concurrency = toml::Value::Integer(DEFAULT_CONCURRENCY as i64);
        let default_timeout = toml::Value::Integer(DEFAULT_TIMEOUT_SECS as i64);
        let empty = toml::Table::new();
        let api = self
            .root
            .get("api")
            .and_then(|v| v.as_table())
            .unwrap_or(&empty);
        out.push_str("[api]\n");
        let endpoint = api.get("endpoint").unwrap_or(&default_endpoint);
        let concurrency = api.get("concurrency").unwrap_or(&default_concurrency);
        let timeout = api.get("timeout_secs").unwrap_or(&default_timeout);
        out.push_str(&format!("endpoint = {}\n", toml_value_display(endpoint)));
        out.push_str(&format!(
            "concurrency = {}\n",
            toml_value_display(concurrency)
        ));
        out.push_str(&format!("timeout_secs = {}\n", toml_value_display(timeout)));
        out
    }
}

fn ensure_table<'a>(root: &'a mut toml::Table, key: &str) -> &'a mut toml::Table {
    if !root.get(key).map(|v| v.is_table()).unwrap_or(false) {
        root.insert(key.to_owned(), toml::Value::Table(toml::Table::new()));
    }
    root.get_mut(key)
        .and_then(|v| v.as_table_mut())
        .expect("inserted above")
}

fn append_table(out: &mut String, table: &toml::Table) {
    for (k, v) in table {
        out.push_str(&format!("{k} = {}\n", toml_value_display(v)));
    }
}

fn toml_value_display(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => format!("{s:?}"),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        other => other.to_string(),
    }
}
