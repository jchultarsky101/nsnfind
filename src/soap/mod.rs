//! SOAP 1.1 support for the upstream parts-availability backend.
//!
//! Each operation lives in its own submodule with its own request/response types
//! and its own `build_request` / `parse_response` pair. Shared pieces — the
//! envelope wrapper used to detect a server-level `soap:Fault`, basic XML
//! escaping, and the service namespace — live here.

use serde::Deserialize;

use crate::error::IlsError;

pub mod availability;
pub mod government;

pub const SERVICE_NS: &str = "http://namespace.ilsmart.com/v2";
pub const MS_ARRAYS_NS: &str = "http://schemas.microsoft.com/2003/10/Serialization/Arrays";

/// Escape the five XML special characters. Used for all user-provided values
/// that are spliced into a request body.
pub fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// SOAP 1.1 envelope-level fault (as opposed to the service-level `<Faults>`
/// collection that some operations return inside their own response body).
#[derive(Debug, Deserialize)]
pub(crate) struct SoapFault {
    pub(crate) faultcode: Option<String>,
    pub(crate) faultstring: Option<String>,
}

/// If the given body element contains a `<soap:Fault>`, convert it to an
/// `IlsError::SoapFault`. Each operation's parser calls this before inspecting
/// its own response shape.
pub(crate) fn fault_to_error(fault: SoapFault) -> IlsError {
    IlsError::SoapFault {
        code: fault.faultcode.unwrap_or_default(),
        message: fault.faultstring.unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_xml_special_chars() {
        assert_eq!(
            xml_escape(r#"a&b<c>d"e'f"#),
            "a&amp;b&lt;c&gt;d&quot;e&apos;f"
        );
    }
}
