use thiserror::Error;

#[derive(Debug, Error)]
pub enum IlsError {
    #[error("http transport: {0}")]
    Http(#[from] reqwest::Error),

    #[error("unexpected HTTP status {status}: {body}")]
    Status { status: u16, body: String },

    #[error("malformed SOAP response: {0}")]
    Parse(String),

    #[error("SOAP fault {code}: {message}")]
    SoapFault { code: String, message: String },

    #[error("invalid NSN '{input}': {reason}")]
    InvalidNsn { input: String, reason: &'static str },
}
