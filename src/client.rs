use std::time::Duration;

use futures::stream::{self, StreamExt};
use tracing::{debug, warn};

use crate::config::Config;
use crate::error::IlsError;
use crate::nsn::InputEntry;
use crate::soap::availability::{self, Availability};
use crate::soap::government::{self, Dataset, GovernmentResult};

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

    pub async fn get_government_data(
        &self,
        part_number: &str,
        datasets: &[Dataset],
    ) -> Result<GovernmentResult, IlsError> {
        let body = government::build_request(&self.user_id, &self.password, part_number, datasets);
        let text = self
            .post_soap(
                government::SOAP_ACTION,
                body,
                "GetGovernmentData",
                part_number,
            )
            .await?;
        government::parse_response(&text)
    }

    /// Combined flow: look up government data first; if it indicates the part
    /// has marketplace listings, also call `GetPartsAvailability` and merge.
    pub async fn lookup(
        &self,
        part_number: &str,
        gov_datasets: &[Dataset],
    ) -> Result<Combined, IlsError> {
        let government = self.get_government_data(part_number, gov_datasets).await?;
        if !government.faults.is_empty() || government.items.is_empty() {
            return Ok(Combined {
                government,
                availability: None,
                availability_error: None,
            });
        }
        match government.has_parts_availability() {
            Some(true) => match self.get_parts_availability(part_number).await {
                Ok(a) => Ok(Combined {
                    government,
                    availability: Some(a),
                    availability_error: None,
                }),
                Err(e) => {
                    warn!(
                        part = part_number,
                        error = %e,
                        "availability lookup failed after successful government lookup"
                    );
                    Ok(Combined {
                        government,
                        availability: None,
                        availability_error: Some(e.to_string()),
                    })
                }
            },
            _ => Ok(Combined {
                government,
                availability: None,
                availability_error: None,
            }),
        }
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

    pub async fn run_government(
        &self,
        entries: Vec<InputEntry>,
        datasets: Vec<Dataset>,
    ) -> Vec<QueryResult<GovernmentResult>> {
        let datasets = &datasets;
        let futs = entries.into_iter().enumerate().map(|(idx, entry)| async move {
            let outcome = match entry.parsed.as_ref() {
                Ok(nsn) => match self.get_government_data(&nsn.normalized, datasets).await {
                    Ok(v) => Outcome::Ok(v),
                    Err(e) => {
                        warn!(op = "government", part = %nsn.normalized, error = %e, "query failed");
                        Outcome::Err(e.to_string())
                    }
                },
                Err(e) => Outcome::Invalid(e.to_string()),
            };
            (idx, QueryResult { entry, outcome })
        });
        collect_sorted(futs, self.concurrency).await
    }

    pub async fn run_lookup(
        &self,
        entries: Vec<InputEntry>,
        gov_datasets: Vec<Dataset>,
    ) -> Vec<QueryResult<Combined>> {
        let datasets = &gov_datasets;
        let futs = entries.into_iter().enumerate().map(|(idx, entry)| async move {
            let outcome = match entry.parsed.as_ref() {
                Ok(nsn) => match self.lookup(&nsn.normalized, datasets).await {
                    Ok(v) => Outcome::Ok(v),
                    Err(e) => {
                        warn!(op = "lookup", part = %nsn.normalized, error = %e, "query failed");
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
pub struct Combined {
    pub government: GovernmentResult,
    pub availability: Option<Availability>,
    pub availability_error: Option<String>,
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
