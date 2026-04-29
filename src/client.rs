use std::time::Duration;

use futures::stream::{self, StreamExt};
use tracing::{debug, warn};

use crate::config::Config;
use crate::error::IlsError;
use crate::nsn::InputEntry;
use crate::soap::availability::{self, Availability};

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

    pub async fn get_parts_availability(
        &self,
        part_number: &str,
    ) -> Result<Availability, IlsError> {
        let body = availability::build_request(&self.user_id, &self.password, part_number);
        let text = self
            .post_soap(
                availability::SOAP_ACTION,
                body,
                "GetPartsAvailability",
                part_number,
            )
            .await?;
        availability::parse_response(&text)
    }

    async fn post_soap(
        &self,
        action: &str,
        body: String,
        op: &str,
        part_number: &str,
    ) -> Result<String, IlsError> {
        debug!(
            op,
            part = part_number,
            bytes = body.len(),
            "POST SOAP request"
        );
        let resp = self
            .http
            .post(&self.endpoint)
            .header("Content-Type", "text/xml; charset=utf-8")
            .header("SOAPAction", format!("\"{action}\""))
            .body(body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        debug!(
            op,
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
        Ok(text)
    }

    pub async fn run_availability(
        &self,
        entries: Vec<InputEntry>,
    ) -> Vec<QueryResult<Availability>> {
        let futs = entries.into_iter().enumerate().map(|(idx, entry)| async move {
            let outcome = match entry.parsed.as_ref() {
                Ok(nsn) => match self.get_parts_availability(&nsn.normalized).await {
                    Ok(v) => Outcome::Ok(v),
                    Err(e) => {
                        warn!(op = "availability", part = %nsn.normalized, error = %e, "query failed");
                        Outcome::Err(e.to_string())
                    }
                },
                Err(e) => Outcome::Invalid(e.to_string()),
            };
            (idx, QueryResult { entry, outcome })
        });
        collect_sorted(futs, self.concurrency).await
    }
}

async fn collect_sorted<T, I, F>(futs: I, concurrency: usize) -> Vec<QueryResult<T>>
where
    I: Iterator<Item = F>,
    F: std::future::Future<Output = (usize, QueryResult<T>)>,
{
    let mut collected: Vec<_> = stream::iter(futs)
        .buffer_unordered(concurrency)
        .collect()
        .await;
    collected.sort_by_key(|(i, _)| *i);
    collected.into_iter().map(|(_, r)| r).collect()
}

#[derive(Debug)]
pub struct QueryResult<T> {
    pub entry: InputEntry,
    pub outcome: Outcome<T>,
}

#[derive(Debug)]
pub enum Outcome<T> {
    Ok(T),
    Err(String),
    Invalid(String),
}
