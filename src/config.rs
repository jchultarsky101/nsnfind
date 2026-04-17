use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

pub const DEFAULT_ENDPOINT: &str = "https://secure.ilsmart.com/services/v2/soap11";
pub const DEFAULT_CONCURRENCY: usize = 4;
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub credentials: Credentials,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(skip)]
    pub source: Option<PathBuf>,
}

#[derive(Deserialize)]
pub struct Credentials {
    pub user_id: String,
    pub password: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("user_id", &self.user_id)
            .field("password", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            endpoint: default_endpoint(),
            concurrency: default_concurrency(),
            timeout_secs: default_timeout(),
        }
    }
}

fn default_endpoint() -> String {
    DEFAULT_ENDPOINT.to_owned()
}
fn default_concurrency() -> usize {
    DEFAULT_CONCURRENCY
}
fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

impl Config {
    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        let path = Self::resolve_path(explicit).ok_or_else(|| {
            anyhow!(
                "no config file found; pass --config, set $NSNFIND_CONFIG, \
                 or create ./nsnfind.toml or $HOME/.config/nsnfind/config.toml"
            )
        })?;
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        let mut cfg: Config =
            toml::from_str(&text).with_context(|| format!("invalid TOML in {}", path.display()))?;
        cfg.source = Some(path);
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn resolve_existing(explicit: Option<&Path>) -> Option<PathBuf> {
        Self::resolve_path(explicit)
    }

    fn resolve_path(explicit: Option<&Path>) -> Option<PathBuf> {
        if let Some(p) = explicit {
            return Some(p.to_owned());
        }
        if let Ok(p) = std::env::var("NSNFIND_CONFIG")
            && !p.is_empty()
        {
            return Some(PathBuf::from(p));
        }
        let cwd = PathBuf::from("nsnfind.toml");
        if cwd.is_file() {
            return Some(cwd);
        }
        if let Ok(home) = std::env::var("HOME") {
            let home_cfg = PathBuf::from(home).join(".config/nsnfind/config.toml");
            if home_cfg.is_file() {
                return Some(home_cfg);
            }
        }
        None
    }

    fn validate(&self) -> Result<()> {
        let u = self.credentials.user_id.as_str();
        if !u.ends_with("U01") {
            return Err(anyhow!(
                "credentials.user_id must end with 'U01' (got {u:?})"
            ));
        }
        if u.len() > 10 {
            return Err(anyhow!(
                "credentials.user_id must be at most 10 characters (got {})",
                u.len()
            ));
        }
        if !u.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(anyhow!("credentials.user_id must be alphanumeric ASCII"));
        }
        let p_len = self.credentials.password.chars().count();
        if !(6..=20).contains(&p_len) {
            return Err(anyhow!(
                "credentials.password must be between 6 and 20 characters (got {p_len})"
            ));
        }
        if self.api.concurrency == 0 {
            return Err(anyhow!("api.concurrency must be >= 1"));
        }
        if self.api.timeout_secs == 0 {
            return Err(anyhow!("api.timeout_secs must be >= 1"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_config_parses() {
        let text = r#"
[credentials]
user_id = "ABCU01"
password = "s3cret"
"#;
        let cfg: Config = toml::from_str(text).expect("parse");
        assert_eq!(cfg.credentials.user_id, "ABCU01");
        assert_eq!(cfg.api.endpoint, DEFAULT_ENDPOINT);
        assert_eq!(cfg.api.concurrency, DEFAULT_CONCURRENCY);
        cfg.validate().expect("valid");
    }

    #[test]
    fn validate_rejects_bad_user_id_suffix() {
        let text = r#"
[credentials]
user_id = "ABCDEF"
password = "s3cret"
"#;
        let cfg: Config = toml::from_str(text).unwrap();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_short_password() {
        let text = r#"
[credentials]
user_id = "ABCU01"
password = "short"
"#;
        let cfg: Config = toml::from_str(text).unwrap();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn credentials_debug_redacts_password() {
        let c = Credentials {
            user_id: "ABCU01".into(),
            password: "topsecret".into(),
        };
        let s = format!("{c:?}");
        assert!(s.contains("ABCU01"));
        assert!(!s.contains("topsecret"));
        assert!(s.contains("redacted"));
    }
}
