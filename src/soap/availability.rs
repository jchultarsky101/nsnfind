//! `GetPartsAvailability` — marketplace listings for a single part number.

use serde::{Deserialize, Serialize};

use super::{SERVICE_NS, SoapFault, fault_to_error, xml_escape};
use crate::error::IlsError;

pub const SOAP_ACTION: &str = "http://namespace.ilsmart.com/v2/GetPartsAvailability";

pub fn build_request(user_id: &str, password: &str, part_number: &str) -> String {
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="utf-8"?>"#,
            "\n",
            r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">"#,
            "\n",
            "  <s:Body>\n",
            r#"    <GetPartsAvailability xmlns="{ns}">"#,
            "\n",
            "      <PartNumber>{pn}</PartNumber>\n",
            "      <Password>{pw}</Password>\n",
            "      <UserId>{uid}</UserId>\n",
            "    </GetPartsAvailability>\n",
            "  </s:Body>\n",
            "</s:Envelope>\n",
        ),
        ns = SERVICE_NS,
        pn = xml_escape(part_number),
        pw = xml_escape(password),
        uid = xml_escape(user_id),
    )
}

pub fn parse_response(xml: &str) -> Result<Availability, IlsError> {
    let env: Envelope = quick_xml::de::from_str(xml)
        .map_err(|e| IlsError::Parse(format!("xml deserialize: {e}")))?;
    if let Some(fault) = env.body.fault {
        return Err(fault_to_error(fault));
    }
    let resp = env.body.response.ok_or_else(|| {
        IlsError::Parse("missing GetPartsAvailabilityResponse element".to_owned())
    })?;
    let body = resp.body.unwrap_or_default();
    Ok(Availability {
        faults: body.faults.map(|f| f.items).unwrap_or_default(),
        part_listings: body.part_listings.map(|p| p.items).unwrap_or_default(),
    })
}

#[derive(Debug)]
pub struct Availability {
    pub faults: Vec<Fault>,
    pub part_listings: Vec<PartListing>,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(rename = "Body")]
    body: SoapBody,
}

#[derive(Debug, Default, Deserialize)]
struct SoapBody {
    #[serde(rename = "GetPartsAvailabilityResponse", default)]
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
    #[serde(rename = "PartListings", default)]
    part_listings: Option<PartListingsWrapper>,
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
struct PartListingsWrapper {
    #[serde(rename = "PartListings", default)]
    items: Vec<PartListing>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartListing {
    #[serde(rename = "Company", default, skip_serializing_if = "Option::is_none")]
    pub company: Option<Company>,
    #[serde(rename = "Parts", default, skip_serializing_if = "Option::is_none")]
    pub parts: Option<PartsWrapper>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Company {
    #[serde(rename = "Id", default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "Name", default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        rename = "SupplierCAGE",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub supplier_cage: Option<String>,
    #[serde(
        rename = "AccreditedVendorLevel",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub accredited_vendor_level: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartsWrapper {
    #[serde(rename = "PartSearchResult", default)]
    pub items: Vec<PartSearchResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartSearchResult {
    #[serde(
        rename = "AlternatePartNumber",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub alternate_part_number: Option<String>,
    #[serde(
        rename = "ConditionCode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub condition_code: Option<String>,
    #[serde(
        rename = "Description",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(
        rename = "ExchangeOption",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exchange_option: Option<String>,
    #[serde(
        rename = "IsGListing",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub is_g_listing: Option<bool>,
    #[serde(
        rename = "IsMListing",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub is_m_listing: Option<bool>,
    #[serde(
        rename = "IsPreferredVendor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub is_preferred_vendor: Option<bool>,
    #[serde(rename = "Maker", default, skip_serializing_if = "Option::is_none")]
    pub maker: Option<String>,
    #[serde(rename = "Model", default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(
        rename = "PartEntered",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub part_entered: Option<String>,
    #[serde(
        rename = "PartNumber",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub part_number: Option<String>,
    #[serde(rename = "Quantity", default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(
        rename = "SearchPartId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub search_part_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_contains_expected_fields() {
        let req = build_request("ABCU01", "s3cret", "4730012345678");
        assert!(req.contains(r#"<GetPartsAvailability xmlns="http://namespace.ilsmart.com/v2">"#));
        assert!(req.contains("<PartNumber>4730012345678</PartNumber>"));
        assert!(req.contains("<Password>s3cret</Password>"));
        assert!(req.contains("<UserId>ABCU01</UserId>"));
    }

    #[test]
    fn request_escapes_values() {
        let req = build_request("u<1>", "p&p", "A\"B");
        assert!(req.contains("<UserId>u&lt;1&gt;</UserId>"));
        assert!(req.contains("<Password>p&amp;p</Password>"));
        assert!(req.contains("<PartNumber>A&quot;B</PartNumber>"));
    }

    #[test]
    fn parses_empty_success_response() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <GetPartsAvailabilityResponse xmlns="http://namespace.ilsmart.com/v2">
      <Body/>
    </GetPartsAvailabilityResponse>
  </s:Body>
</s:Envelope>"#;
        let a = parse_response(xml).expect("parse ok");
        assert!(a.faults.is_empty());
        assert!(a.part_listings.is_empty());
    }

    #[test]
    fn parses_populated_response() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <GetPartsAvailabilityResponse xmlns="http://namespace.ilsmart.com/v2">
      <Body>
        <PartListings>
          <PartListings>
            <Company>
              <Id>ABCD</Id>
              <Name>Acme Aviation</Name>
              <SupplierCAGE>1K2L3</SupplierCAGE>
            </Company>
            <Parts>
              <PartSearchResult>
                <ConditionCode>NE</ConditionCode>
                <PartNumber>123-456</PartNumber>
                <PartEntered>4730012345678</PartEntered>
                <Quantity>7</Quantity>
                <IsPreferredVendor>false</IsPreferredVendor>
              </PartSearchResult>
            </Parts>
          </PartListings>
        </PartListings>
      </Body>
    </GetPartsAvailabilityResponse>
  </s:Body>
</s:Envelope>"#;
        let a = parse_response(xml).expect("parse ok");
        assert_eq!(a.part_listings.len(), 1);
        let listing = &a.part_listings[0];
        let company = listing.company.as_ref().expect("company");
        assert_eq!(company.name.as_deref(), Some("Acme Aviation"));
        assert_eq!(company.supplier_cage.as_deref(), Some("1K2L3"));
        let parts = listing.parts.as_ref().expect("parts");
        assert_eq!(parts.items.len(), 1);
        let p = &parts.items[0];
        assert_eq!(p.part_number.as_deref(), Some("123-456"));
        assert_eq!(p.condition_code.as_deref(), Some("NE"));
        assert_eq!(p.quantity.as_deref(), Some("7"));
        assert_eq!(p.is_preferred_vendor, Some(false));
    }

    #[test]
    fn parses_service_faults() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <GetPartsAvailabilityResponse xmlns="http://namespace.ilsmart.com/v2">
      <Body>
        <Faults>
          <Fault>
            <Message>No parts found</Message>
            <Name>NoResults</Name>
            <Severity>Info</Severity>
          </Fault>
        </Faults>
      </Body>
    </GetPartsAvailabilityResponse>
  </s:Body>
</s:Envelope>"#;
        let a = parse_response(xml).expect("parse ok");
        assert_eq!(a.faults.len(), 1);
        assert_eq!(a.faults[0].message.as_deref(), Some("No parts found"));
    }

    #[test]
    fn detects_soap_envelope_fault() {
        let xml = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <s:Fault>
      <faultcode>s:Client</faultcode>
      <faultstring>Invalid credentials</faultstring>
    </s:Fault>
  </s:Body>
</s:Envelope>"#;
        match parse_response(xml) {
            Err(IlsError::SoapFault { code, message }) => {
                assert_eq!(code, "s:Client");
                assert_eq!(message, "Invalid credentials");
            }
            other => panic!("expected SoapFault, got {other:?}"),
        }
    }
}
