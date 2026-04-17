use std::time::Duration;

use futures::stream::{self, StreamExt};
use tracing::{debug, warn};

use crate::config::Config;
use crate::error::IlsError;
use crate::nsn::InputEntry;
use crate::soap::{self, Availability, SOAP_ACTION};

pub struct IlsClient {
    http: reqwest::Client,
    endpoint: String,
    user_id: String,
    password: String,
    concurrency: usize,
}

impl IlsClient {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.api.timeout_secs))
            .user_agent(concat!("nsnfind/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            http,
            endpoint: config.api.endpoint.clone(),
            user_id: config.credentials.user_id.clone(),
            password: config.credentials.password.clone(),
            concurrency: config.api.concurrency,
        })
    }

    pub async fn query_one(&self, part_number: &str) -> Result<Availability, IlsError> {
        let body = soap::build_request(&self.user_id, &self.password, part_number);
        debug!(part = part_number, bytes = body.len(), "POST SOAP request");
        let resp = self
            .http
            .post(&self.endpoint)
            .header("Content-Type", "text/xml; charset=utf-8")
            .header("SOAPAction", format!("\"{SOAP_ACTION}\""))
            .body(body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        debug!(
            part = part_number,
            status = status.as_u16(),
            bytes = text.len(),
            "got SOAP response"
        );
        if !status.is_success() {
            return Err(IlsError::Status {
                status: status.as_u16(),
                body: text,
            });
        }
        soap::parse_response(&text)
    }

    pub async fn query_all(&self, entries: Vec<InputEntry>) -> Vec<QueryResult> {
        let concurrency = self.concurrency;
        let futs = entries
            .into_iter()
            .enumerate()
            .map(|(idx, entry)| async move {
                let outcome = match entry.parsed.as_ref() {
                    Ok(nsn) => match self.query_one(&nsn.normalized).await {
                        Ok(a) => Outcome::Ok(a),
                        Err(e) => {
                            warn!(part = %nsn.normalized, error = %e, "query failed");
                            Outcome::Err(e.to_string())
                        }
                    },
                    Err(e) => Outcome::Invalid(e.to_string()),
                };
                (idx, QueryResult { entry, outcome })
            });
        let mut collected: Vec<_> = stream::iter(futs)
            .buffer_unordered(concurrency)
            .collect()
            .await;
        collected.sort_by_key(|(i, _)| *i);
        collected.into_iter().map(|(_, r)| r).collect()
    }
}

#[derive(Debug)]
pub struct QueryResult {
    pub entry: InputEntry,
    pub outcome: Outcome,
}

#[derive(Debug)]
pub enum Outcome {
    Ok(Availability),
    Err(String),
    Invalid(String),
}
