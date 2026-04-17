//! `GetGovernmentData` — DoD reference catalog data for a single NSN/NIIN.
//!
//! The request takes an array of dataset codes (AMDF, CRF, DLA, ISDOD, ISUSAF,
//! MCRL, MLC, MOE, MRIL, NHA, PH, TECH). The response is a grab-bag of optional
//! collections corresponding to the requested datasets. This module models the
//! pieces most useful for "does this NSN exist and what is it" —
//! `ItemName`, `Fsc`, `Niin`, `HasPartsAvailability`, MCRL (manufacturer cross-
//! reference), NSN Info, and Procurement History. Other datasets are parsed
//! leniently and otherwise ignored.

use serde::{Deserialize, Serialize};

use super::{MS_ARRAYS_NS, SERVICE_NS, SoapFault, fault_to_error, xml_escape};
use crate::error::IlsError;

pub const SOAP_ACTION: &str = "http://namespace.ilsmart.com/v2/GetGovernmentData";

/// Dataset codes accepted by `GovFilesToSearch`. At least one must be passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dataset {
    Amdf,
    Crf,
    Dla,
    Isdod,
    Isusaf,
    Mcrl,
    Mlc,
    Moe,
    Mril,
    Nha,
    Ph,
    Tech,
}

impl Dataset {
    pub fn code(&self) -> &'static str {
        match self {
            Dataset::Amdf => "AMDF",
            Dataset::Crf => "CRF",
            Dataset::Dla => "DLA",
            Dataset::Isdod => "ISDOD",
            Dataset::Isusaf => "ISUSAF",
            Dataset::Mcrl => "MCRL",
            Dataset::Mlc => "MLC",
            Dataset::Moe => "MOE",
            Dataset::Mril => "MRIL",
            Dataset::Nha => "NHA",
            Dataset::Ph => "PH",
            Dataset::Tech => "TECH",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_uppercase().as_str() {
            "AMDF" => Some(Dataset::Amdf),
            "CRF" => Some(Dataset::Crf),
            "DLA" => Some(Dataset::Dla),
            "ISDOD" => Some(Dataset::Isdod),
            "ISUSAF" => Some(Dataset::Isusaf),
            "MCRL" => Some(Dataset::Mcrl),
            "MLC" => Some(Dataset::Mlc),
            "MOE" => Some(Dataset::Moe),
            "MRIL" => Some(Dataset::Mril),
            "NHA" => Some(Dataset::Nha),
            "PH" => Some(Dataset::Ph),
            "TECH" => Some(Dataset::Tech),
            _ => None,
        }
    }
}

pub fn build_request(
    user_id: &str,
    password: &str,
    part_number: &str,
    datasets: &[Dataset],
) -> String {
    assert!(
        !datasets.is_empty(),
        "GetGovernmentData requires at least one dataset; caller must enforce"
    );
    let mut files = String::new();
    for d in datasets {
        files.push_str("        <a:string>");
        files.push_str(d.code());
        files.push_str("</a:string>\n");
    }
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="utf-8"?>"#,
            "\n",
            r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">"#,
            "\n",
            "  <s:Body>\n",
            r#"    <GetGovernmentData xmlns="{ns}">"#,
            "\n",
            r#"      <GovFilesToSearch xmlns:a="{arrays_ns}">"#,
            "\n",
            "{files}",
            "      </GovFilesToSearch>\n",
            "      <PartNumber>{pn}</PartNumber>\n",
            "      <Password>{pw}</Password>\n",
            "      <UserId>{uid}</UserId>\n",
            "    </GetGovernmentData>\n",
            "  </s:Body>\n",
            "</s:Envelope>\n",
        ),
        ns = SERVICE_NS,
        arrays_ns = MS_ARRAYS_NS,
        files = files,
        pn = xml_escape(part_number),
        pw = xml_escape(password),
        uid = xml_escape(user_id),
    )
}

pub fn parse_response(xml: &str) -> Result<GovernmentResult, IlsError> {
    let env: Envelope = quick_xml::de::from_str(xml)
        .map_err(|e| IlsError::Parse(format!("xml deserialize: {e}")))?;
    if let Some(fault) = env.body.fault {
        return Err(fault_to_error(fault));
    }
    let resp = env
        .body
        .response
        .ok_or_else(|| IlsError::Parse("missing GetGovernmentDataResponse element".to_owned()))?;
    let body = resp.body.unwrap_or_default();
    Ok(GovernmentResult {
        faults: body.faults.map(|f| f.items).unwrap_or_default(),
        items: body.search_results.map(|s| s.items).unwrap_or_default(),
    })
}

/// High-level parsed response for one `GetGovernmentData` call.
#[derive(Debug)]
pub struct GovernmentResult {
    pub faults: Vec<Fault>,
    pub items: Vec<GovernmentDataSearchResults>,
}

impl GovernmentResult {
    /// First result's `HasPartsAvailability` parsed as a best-effort boolean.
    /// The field is typed `xs:string` in the WSDL; ILS documents "true"/"false"
    /// but we accept common variants case-insensitively.
    pub fn has_parts_availability(&self) -> Option<bool> {
        self.items
            .iter()
            .find_map(|r| r.has_parts_availability.as_deref())
            .and_then(parse_bool_ish)
    }
}

fn parse_bool_ish(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "y" | "1" | "t" => Some(true),
        "false" | "no" | "n" | "0" | "f" | "" => Some(false),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(rename = "Body")]
    body: SoapBody,
}

#[derive(Debug, Default, Deserialize)]
struct SoapBody {
    #[serde(rename = "GetGovernmentDataResponse", default)]
    response: Option<RespWrapper>,
    #[serde(rename = "Fault", default)]
    fault: Option<SoapFault>,
}

#[derive(Debug, Deserialize)]
struct RespWrapper {
    #[serde(rename = "Body", default)]
    body: Option<RespBody>,
}

#[derive(Debug, Default, Deserialize)]
struct RespBody {
    #[serde(rename = "Faults", default)]
    faults: Option<FaultsWrapper>,
    #[serde(rename = "GovernmentSearchResults", default)]
    search_results: Option<SearchResultsWrapper>,
}

#[derive(Debug, Deserialize)]
struct FaultsWrapper {
    #[serde(rename = "Fault", default)]
    items: Vec<Fault>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Fault {
    #[serde(rename = "Message", default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(rename = "Name", default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "Severity", default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(rename = "SubType", default, skip_serializing_if = "Option::is_none")]
    pub sub_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResultsWrapper {
    #[serde(rename = "GovernmentDataSearchResults", default)]
    items: Vec<GovernmentDataSearchResults>,
}

/// Top-level result for one NSN. Most fields are optional because they only
/// appear when the corresponding dataset was requested (and the server had data).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GovernmentDataSearchResults {
    #[serde(rename = "ItemName", default, skip_serializing_if = "Option::is_none")]
    pub item_name: Option<String>,
    #[serde(rename = "Fsc", default, skip_serializing_if = "Option::is_none")]
    pub fsc: Option<String>,
    #[serde(rename = "Niin", default, skip_serializing_if = "Option::is_none")]
    pub niin: Option<String>,
    #[serde(
        rename = "HasPartsAvailability",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub has_parts_availability: Option<String>,
    #[serde(rename = "HistInd", default, skip_serializing_if = "Option::is_none")]
    pub hist_ind: Option<String>,
    #[serde(rename = "McrlData", default, skip_serializing_if = "Option::is_none")]
    pub mcrl_data: Option<Mcrl>,
    #[serde(rename = "NsnInfo", default, skip_serializing_if = "Option::is_none")]
    pub nsn_info: Option<NsnInfo>,
    #[serde(
        rename = "ProcurementHistoryData",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub procurement_history: Option<ProcurementHistoryData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Mcrl {
    #[serde(rename = "McrlItem", default, skip_serializing_if = "Option::is_none")]
    pub items: Option<ArrayOfMcrlItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArrayOfMcrlItem {
    #[serde(rename = "McrlItem", default)]
    pub items: Vec<McrlItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McrlItem {
    #[serde(rename = "Cage", default, skip_serializing_if = "Option::is_none")]
    pub cage: Option<String>,
    #[serde(
        rename = "CompanyName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub company_name: Option<String>,
    #[serde(rename = "Dac", default, skip_serializing_if = "Option::is_none")]
    pub dac: Option<String>,
    #[serde(
        rename = "Historical",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub historical: Option<String>,
    #[serde(
        rename = "PartNumber",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub part_number: Option<String>,
    #[serde(rename = "Rncc", default, skip_serializing_if = "Option::is_none")]
    pub rncc: Option<String>,
    #[serde(rename = "Rnvc", default, skip_serializing_if = "Option::is_none")]
    pub rnvc: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NsnInfo {
    #[serde(
        rename = "NsnInfoItem",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub items: Option<ArrayOfNsnInfoItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArrayOfNsnInfoItem {
    #[serde(rename = "NsnInfoItem", default)]
    pub items: Vec<NsnInfoItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NsnInfoItem {
    #[serde(rename = "AdpCode", default, skip_serializing_if = "Option::is_none")]
    pub adp_code: Option<String>,
    #[serde(
        rename = "DemilitarizationCode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub demilitarization_code: Option<String>,
    #[serde(
        rename = "ItemNameCode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub item_name_code: Option<String>,
    #[serde(rename = "Status", default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcurementHistoryData {
    #[serde(
        rename = "ProcurementEntry",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub entries: Option<ArrayOfProcurementItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArrayOfProcurementItem {
    #[serde(rename = "ProcurementItem", default)]
    pub items: Vec<ProcurementItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcurementItem {
    #[serde(rename = "AwardDate", default, skip_serializing_if = "Option::is_none")]
    pub award_date: Option<String>,
    #[serde(rename = "Cage", default, skip_serializing_if = "Option::is_none")]
    pub cage: Option<String>,
    #[serde(
        rename = "CompanyName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub company_name: Option<String>,
    #[serde(
        rename = "ContractNumber",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub contract_number: Option<String>,
    #[serde(rename = "Quantity", default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_code_roundtrip() {
        for code in [
            "AMDF", "CRF", "DLA", "ISDOD", "ISUSAF", "MCRL", "MLC", "MOE", "MRIL", "NHA", "PH",
            "TECH",
        ] {
            let d = Dataset::parse(code).expect("parse");
            assert_eq!(d.code(), code);
        }
        assert_eq!(Dataset::parse("mcrl"), Some(Dataset::Mcrl));
        assert!(Dataset::parse("UNKNOWN").is_none());
    }

    #[test]
    fn request_contains_expected_fields() {
        let req = build_request(
            "ABCU01",
            "s3cret",
            "4730012345678",
            &[Dataset::Mcrl, Dataset::Ph],
        );
        assert!(req.contains(r#"<GetGovernmentData xmlns="http://namespace.ilsmart.com/v2">"#));
        assert!(req.contains(&format!(r#"<GovFilesToSearch xmlns:a="{}"#, MS_ARRAYS_NS)));
        assert!(req.contains("<a:string>MCRL</a:string>"));
        assert!(req.contains("<a:string>PH</a:string>"));
        assert!(req.contains("<PartNumber>4730012345678</PartNumber>"));
        assert!(req.contains("<Password>s3cret</Password>"));
        assert!(req.contains("<UserId>ABCU01</UserId>"));
    }

    #[test]
    #[should_panic(expected = "requires at least one dataset")]
    fn request_rejects_empty_datasets() {
        build_request("ABCU01", "s3cret", "4730012345678", &[]);
    }

    #[test]
    fn parses_mcrl_response() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <GetGovernmentDataResponse xmlns="http://namespace.ilsmart.com/v2">
      <Body>
        <GovernmentSearchResults>
          <GovernmentDataSearchResults>
            <Fsc>4730</Fsc>
            <HasPartsAvailability>true</HasPartsAvailability>
            <ItemName>VALVE,GLOBE</ItemName>
            <McrlData>
              <McrlItem>
                <McrlItem>
                  <Cage>1K2L3</Cage>
                  <CompanyName>Acme Valve Co</CompanyName>
                  <PartNumber>AC-123</PartNumber>
                </McrlItem>
                <McrlItem>
                  <Cage>99ABC</Cage>
                  <CompanyName>Widget Corp</CompanyName>
                  <PartNumber>W-42</PartNumber>
                </McrlItem>
              </McrlItem>
            </McrlData>
            <Niin>012345678</Niin>
          </GovernmentDataSearchResults>
        </GovernmentSearchResults>
      </Body>
    </GetGovernmentDataResponse>
  </s:Body>
</s:Envelope>"#;
        let r = parse_response(xml).expect("parse ok");
        assert_eq!(r.items.len(), 1);
        let item = &r.items[0];
        assert_eq!(item.item_name.as_deref(), Some("VALVE,GLOBE"));
        assert_eq!(item.fsc.as_deref(), Some("4730"));
        assert_eq!(item.niin.as_deref(), Some("012345678"));
        assert_eq!(item.has_parts_availability.as_deref(), Some("true"));
        let mcrl = item.mcrl_data.as_ref().and_then(|m| m.items.as_ref());
        let mcrl = mcrl.expect("mcrl items present");
        assert_eq!(mcrl.items.len(), 2);
        assert_eq!(mcrl.items[0].cage.as_deref(), Some("1K2L3"));
        assert_eq!(mcrl.items[0].company_name.as_deref(), Some("Acme Valve Co"));
        assert_eq!(r.has_parts_availability(), Some(true));
    }

    #[test]
    fn parses_empty_response() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <GetGovernmentDataResponse xmlns="http://namespace.ilsmart.com/v2">
      <Body/>
    </GetGovernmentDataResponse>
  </s:Body>
</s:Envelope>"#;
        let r = parse_response(xml).expect("parse ok");
        assert!(r.faults.is_empty());
        assert!(r.items.is_empty());
        assert_eq!(r.has_parts_availability(), None);
    }

    #[test]
    fn detects_auth_fault() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <GetGovernmentDataResponse xmlns="http://namespace.ilsmart.com/v2">
      <Body>
        <Faults>
          <Fault>
            <Message>Not entitled</Message>
            <Name>AuthorizationFailure</Name>
            <Severity>Fatal</Severity>
          </Fault>
        </Faults>
      </Body>
    </GetGovernmentDataResponse>
  </s:Body>
</s:Envelope>"#;
        let r = parse_response(xml).expect("parse ok");
        assert_eq!(r.faults.len(), 1);
        assert_eq!(r.faults[0].severity.as_deref(), Some("Fatal"));
    }

    #[test]
    fn bool_ish_parsing() {
        assert_eq!(parse_bool_ish("true"), Some(true));
        assert_eq!(parse_bool_ish("TRUE"), Some(true));
        assert_eq!(parse_bool_ish("Yes"), Some(true));
        assert_eq!(parse_bool_ish("false"), Some(false));
        assert_eq!(parse_bool_ish(""), Some(false));
        assert_eq!(parse_bool_ish("maybe"), None);
    }
}
