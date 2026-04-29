//! Output formatters for each operation. Every entry point takes a `Format`
//! selector, the per-NSN results, and a writer — so the CLI layer doesn't have
//! to know how JSON vs CSV are produced.

use std::io::Write;

use serde::Serialize;

use crate::cli::Format;
use crate::client::{Outcome, QueryResult};
use crate::soap::availability::{Availability, Fault as AvFault, PartListing, PartSearchResult};

// ---------------------------------------------------------------------------
// Availability
// ---------------------------------------------------------------------------

pub fn write_availability<W: Write>(
    format: Format,
    results: &[QueryResult<Availability>],
    writer: W,
) -> anyhow::Result<()> {
    match format {
        Format::Json => write_availability_json(results, writer),
        Format::Csv => write_availability_csv(results, writer),
    }
}

fn write_availability_json<W: Write>(
    results: &[QueryResult<Availability>],
    writer: W,
) -> anyhow::Result<()> {
    let records: Vec<AvailabilityJson<'_>> = results.iter().map(AvailabilityJson::from).collect();
    serde_json::to_writer_pretty(writer, &records)?;
    Ok(())
}

#[derive(Serialize)]
struct AvailabilityJson<'a> {
    line: usize,
    input: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    normalized: Option<&'a str>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    faults: Vec<AvFault>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    listings: Vec<PartListing>,
}

impl<'a> From<&'a QueryResult<Availability>> for AvailabilityJson<'a> {
    fn from(r: &'a QueryResult<Availability>) -> Self {
        let normalized = normalized_of(&r.entry);
        match &r.outcome {
            Outcome::Ok(a) => AvailabilityJson {
                line: r.entry.line,
                input: &r.entry.raw,
                normalized,
                status: availability_status(a),
                error: None,
                faults: a.faults.clone(),
                listings: a.part_listings.clone(),
            },
            Outcome::Err(e) => AvailabilityJson {
                line: r.entry.line,
                input: &r.entry.raw,
                normalized,
                status: "error",
                error: Some(e.clone()),
                faults: Vec::new(),
                listings: Vec::new(),
            },
            Outcome::Invalid(e) => AvailabilityJson {
                line: r.entry.line,
                input: &r.entry.raw,
                normalized: None,
                status: "invalid",
                error: Some(e.clone()),
                faults: Vec::new(),
                listings: Vec::new(),
            },
        }
    }
}

fn availability_status(a: &Availability) -> &'static str {
    match (!a.part_listings.is_empty(), !a.faults.is_empty()) {
        (true, _) => "ok",
        (false, true) => "api_fault",
        (false, false) => "no_results",
    }
}

const AVAILABILITY_CSV_HEADERS: [&str; 22] = [
    "line",
    "input",
    "normalized",
    "status",
    "error",
    "company_id",
    "company_name",
    "supplier_cage",
    "accredited_vendor_level",
    "part_number",
    "alternate_part_number",
    "condition_code",
    "description",
    "exchange_option",
    "quantity",
    "maker",
    "model",
    "part_entered",
    "search_part_id",
    "is_preferred_vendor",
    "is_g_listing",
    "is_m_listing",
];

fn write_availability_csv<W: Write>(
    results: &[QueryResult<Availability>],
    writer: W,
) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_writer(writer);
    wtr.write_record(AVAILABILITY_CSV_HEADERS)?;
    for r in results {
        let line = r.entry.line.to_string();
        let input = r.entry.raw.as_str();
        let normalized = normalized_str(&r.entry);
        match &r.outcome {
            Outcome::Ok(a) => {
                if a.part_listings.is_empty() {
                    let status = if a.faults.is_empty() {
                        "no_results"
                    } else {
                        "api_fault"
                    };
                    let err =
                        join_fault_messages(a.faults.iter().filter_map(|f| f.message.as_deref()));
                    wtr.write_record(avail_summary_row(&line, input, normalized, status, &err))?;
                    continue;
                }
                let mut any_row = false;
                for listing in &a.part_listings {
                    let items = listing
                        .parts
                        .as_ref()
                        .map(|p| p.items.as_slice())
                        .unwrap_or(&[]);
                    if items.is_empty() {
                        any_row = true;
                        wtr.write_record(avail_listing_only_row(
                            &line, input, normalized, listing,
                        ))?;
                    } else {
                        for part in items {
                            any_row = true;
                            wtr.write_record(avail_full_row(
                                &line, input, normalized, listing, part,
                            ))?;
                        }
                    }
                }
                if !any_row {
                    wtr.write_record(avail_summary_row(
                        &line,
                        input,
                        normalized,
                        "no_results",
                        "",
                    ))?;
                }
            }
            Outcome::Err(e) => {
                wtr.write_record(avail_summary_row(&line, input, normalized, "error", e))?;
            }
            Outcome::Invalid(e) => {
                wtr.write_record(avail_summary_row(&line, input, "", "invalid", e))?;
            }
        }
    }
    wtr.flush()?;
    Ok(())
}

fn avail_summary_row<'a>(
    line: &'a str,
    input: &'a str,
    normalized: &'a str,
    status: &'a str,
    error: &'a str,
) -> [&'a str; 22] {
    [
        line, input, normalized, status, error, "", "", "", "", "", "", "", "", "", "", "", "", "",
        "", "", "", "",
    ]
}

fn avail_listing_only_row<'a>(
    line: &'a str,
    input: &'a str,
    normalized: &'a str,
    listing: &'a PartListing,
) -> [&'a str; 22] {
    let company = listing.company.as_ref();
    [
        line,
        input,
        normalized,
        "ok",
        "",
        company.and_then(|c| c.id.as_deref()).unwrap_or(""),
        company.and_then(|c| c.name.as_deref()).unwrap_or(""),
        company
            .and_then(|c| c.supplier_cage.as_deref())
            .unwrap_or(""),
        company
            .and_then(|c| c.accredited_vendor_level.as_deref())
            .unwrap_or(""),
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
    ]
}

fn avail_full_row<'a>(
    line: &'a str,
    input: &'a str,
    normalized: &'a str,
    listing: &'a PartListing,
    part: &'a PartSearchResult,
) -> [&'a str; 22] {
    let company = listing.company.as_ref();
    [
        line,
        input,
        normalized,
        "ok",
        "",
        company.and_then(|c| c.id.as_deref()).unwrap_or(""),
        company.and_then(|c| c.name.as_deref()).unwrap_or(""),
        company
            .and_then(|c| c.supplier_cage.as_deref())
            .unwrap_or(""),
        company
            .and_then(|c| c.accredited_vendor_level.as_deref())
            .unwrap_or(""),
        part.part_number.as_deref().unwrap_or(""),
        part.alternate_part_number.as_deref().unwrap_or(""),
        part.condition_code.as_deref().unwrap_or(""),
        part.description.as_deref().unwrap_or(""),
        part.exchange_option.as_deref().unwrap_or(""),
        part.quantity.as_deref().unwrap_or(""),
        part.maker.as_deref().unwrap_or(""),
        part.model.as_deref().unwrap_or(""),
        part.part_entered.as_deref().unwrap_or(""),
        part.search_part_id.as_deref().unwrap_or(""),
        bool_str(part.is_preferred_vendor),
        bool_str(part.is_g_listing),
        bool_str(part.is_m_listing),
    ]
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn bool_str(b: Option<bool>) -> &'static str {
    match b {
        Some(true) => "true",
        Some(false) => "false",
        None => "",
    }
}

fn normalized_of(entry: &crate::nsn::InputEntry) -> Option<&str> {
    entry.parsed.as_ref().ok().map(|n| n.normalized.as_str())
}

fn normalized_str(entry: &crate::nsn::InputEntry) -> &str {
    normalized_of(entry).unwrap_or("")
}

fn join_fault_messages<'a, I: IntoIterator<Item = &'a str>>(iter: I) -> String {
    iter.into_iter().collect::<Vec<_>>().join("; ")
}
