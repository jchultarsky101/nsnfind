//! Output formatters for each operation. Every entry point takes a `Format`
//! selector, the per-NSN results, and a writer — so the CLI layer doesn't have
//! to know how JSON vs CSV are produced.

use std::io::Write;

use serde::Serialize;

use crate::cli::Format;
use crate::client::{Combined, Outcome, QueryResult};
use crate::soap::availability::{Availability, Fault as AvFault, PartListing, PartSearchResult};
use crate::soap::government::{
    Fault as GovFault, GovernmentDataSearchResults, GovernmentResult, McrlItem,
};

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
// Government
// ---------------------------------------------------------------------------

pub fn write_government<W: Write>(
    format: Format,
    results: &[QueryResult<GovernmentResult>],
    writer: W,
) -> anyhow::Result<()> {
    match format {
        Format::Json => write_government_json(results, writer),
        Format::Csv => write_government_csv(results, writer),
    }
}

fn write_government_json<W: Write>(
    results: &[QueryResult<GovernmentResult>],
    writer: W,
) -> anyhow::Result<()> {
    let records: Vec<GovernmentJson<'_>> = results.iter().map(GovernmentJson::from).collect();
    serde_json::to_writer_pretty(writer, &records)?;
    Ok(())
}

#[derive(Serialize)]
struct GovernmentJson<'a> {
    line: usize,
    input: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    normalized: Option<&'a str>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    faults: Vec<GovFault>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    items: Vec<GovernmentDataSearchResults>,
}

impl<'a> From<&'a QueryResult<GovernmentResult>> for GovernmentJson<'a> {
    fn from(r: &'a QueryResult<GovernmentResult>) -> Self {
        let normalized = normalized_of(&r.entry);
        match &r.outcome {
            Outcome::Ok(g) => GovernmentJson {
                line: r.entry.line,
                input: &r.entry.raw,
                normalized,
                status: government_status(g),
                error: None,
                faults: g.faults.clone(),
                items: g.items.clone(),
            },
            Outcome::Err(e) => GovernmentJson {
                line: r.entry.line,
                input: &r.entry.raw,
                normalized,
                status: "error",
                error: Some(e.clone()),
                faults: Vec::new(),
                items: Vec::new(),
            },
            Outcome::Invalid(e) => GovernmentJson {
                line: r.entry.line,
                input: &r.entry.raw,
                normalized: None,
                status: "invalid",
                error: Some(e.clone()),
                faults: Vec::new(),
                items: Vec::new(),
            },
        }
    }
}

fn government_status(g: &GovernmentResult) -> &'static str {
    match (!g.items.is_empty(), !g.faults.is_empty()) {
        (true, _) => "ok",
        (false, true) => "api_fault",
        (false, false) => "no_results",
    }
}

const GOVERNMENT_CSV_HEADERS: [&str; 12] = [
    "line",
    "input",
    "normalized",
    "status",
    "error",
    "item_name",
    "fsc",
    "niin",
    "has_parts_availability",
    "mcrl_cage",
    "mcrl_company_name",
    "mcrl_part_number",
];

fn write_government_csv<W: Write>(
    results: &[QueryResult<GovernmentResult>],
    writer: W,
) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_writer(writer);
    wtr.write_record(GOVERNMENT_CSV_HEADERS)?;
    for r in results {
        let line = r.entry.line.to_string();
        let input = r.entry.raw.as_str();
        let normalized = normalized_str(&r.entry);
        match &r.outcome {
            Outcome::Ok(g) => {
                if g.items.is_empty() {
                    let status = if g.faults.is_empty() {
                        "no_results"
                    } else {
                        "api_fault"
                    };
                    let err =
                        join_fault_messages(g.faults.iter().filter_map(|f| f.message.as_deref()));
                    wtr.write_record(gov_summary_row(&line, input, normalized, status, &err))?;
                    continue;
                }
                for item in &g.items {
                    let mcrl_items = item
                        .mcrl_data
                        .as_ref()
                        .and_then(|m| m.items.as_ref())
                        .map(|a| a.items.as_slice())
                        .unwrap_or(&[]);
                    if mcrl_items.is_empty() {
                        wtr.write_record(gov_item_only_row(&line, input, normalized, item))?;
                    } else {
                        for mcrl in mcrl_items {
                            wtr.write_record(gov_full_row(&line, input, normalized, item, mcrl))?;
                        }
                    }
                }
            }
            Outcome::Err(e) => {
                wtr.write_record(gov_summary_row(&line, input, normalized, "error", e))?;
            }
            Outcome::Invalid(e) => {
                wtr.write_record(gov_summary_row(&line, input, "", "invalid", e))?;
            }
        }
    }
    wtr.flush()?;
    Ok(())
}

fn gov_summary_row<'a>(
    line: &'a str,
    input: &'a str,
    normalized: &'a str,
    status: &'a str,
    error: &'a str,
) -> [&'a str; 12] {
    [
        line, input, normalized, status, error, "", "", "", "", "", "", "",
    ]
}

fn gov_item_only_row<'a>(
    line: &'a str,
    input: &'a str,
    normalized: &'a str,
    item: &'a GovernmentDataSearchResults,
) -> [&'a str; 12] {
    [
        line,
        input,
        normalized,
        "ok",
        "",
        item.item_name.as_deref().unwrap_or(""),
        item.fsc.as_deref().unwrap_or(""),
        item.niin.as_deref().unwrap_or(""),
        item.has_parts_availability.as_deref().unwrap_or(""),
        "",
        "",
        "",
    ]
}

fn gov_full_row<'a>(
    line: &'a str,
    input: &'a str,
    normalized: &'a str,
    item: &'a GovernmentDataSearchResults,
    mcrl: &'a McrlItem,
) -> [&'a str; 12] {
    [
        line,
        input,
        normalized,
        "ok",
        "",
        item.item_name.as_deref().unwrap_or(""),
        item.fsc.as_deref().unwrap_or(""),
        item.niin.as_deref().unwrap_or(""),
        item.has_parts_availability.as_deref().unwrap_or(""),
        mcrl.cage.as_deref().unwrap_or(""),
        mcrl.company_name.as_deref().unwrap_or(""),
        mcrl.part_number.as_deref().unwrap_or(""),
    ]
}

// ---------------------------------------------------------------------------
// Combined (lookup)
// ---------------------------------------------------------------------------

pub fn write_combined<W: Write>(
    format: Format,
    results: &[QueryResult<Combined>],
    writer: W,
) -> anyhow::Result<()> {
    match format {
        Format::Json => write_combined_json(results, writer),
        Format::Csv => write_combined_csv(results, writer),
    }
}

fn write_combined_json<W: Write>(
    results: &[QueryResult<Combined>],
    writer: W,
) -> anyhow::Result<()> {
    let records: Vec<CombinedJson<'_>> = results.iter().map(CombinedJson::from).collect();
    serde_json::to_writer_pretty(writer, &records)?;
    Ok(())
}

#[derive(Serialize)]
struct CombinedJson<'a> {
    line: usize,
    input: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    normalized: Option<&'a str>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    availability_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    government: Option<GovernmentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    availability: Option<AvailabilityBlock>,
}

#[derive(Serialize)]
struct GovernmentBlock {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    faults: Vec<GovFault>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    items: Vec<GovernmentDataSearchResults>,
}

#[derive(Serialize)]
struct AvailabilityBlock {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    faults: Vec<AvFault>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    listings: Vec<PartListing>,
}

impl<'a> From<&'a QueryResult<Combined>> for CombinedJson<'a> {
    fn from(r: &'a QueryResult<Combined>) -> Self {
        let normalized = normalized_of(&r.entry);
        match &r.outcome {
            Outcome::Ok(c) => CombinedJson {
                line: r.entry.line,
                input: &r.entry.raw,
                normalized,
                status: combined_status(c),
                error: None,
                availability_error: c.availability_error.clone(),
                government: Some(GovernmentBlock {
                    faults: c.government.faults.clone(),
                    items: c.government.items.clone(),
                }),
                availability: c.availability.as_ref().map(|a| AvailabilityBlock {
                    faults: a.faults.clone(),
                    listings: a.part_listings.clone(),
                }),
            },
            Outcome::Err(e) => CombinedJson {
                line: r.entry.line,
                input: &r.entry.raw,
                normalized,
                status: "error",
                error: Some(e.clone()),
                availability_error: None,
                government: None,
                availability: None,
            },
            Outcome::Invalid(e) => CombinedJson {
                line: r.entry.line,
                input: &r.entry.raw,
                normalized: None,
                status: "invalid",
                error: Some(e.clone()),
                availability_error: None,
                government: None,
                availability: None,
            },
        }
    }
}

fn combined_status(c: &Combined) -> &'static str {
    if !c.government.faults.is_empty() {
        return "gov_fault";
    }
    if c.government.items.is_empty() {
        return "not_in_catalog";
    }
    match c.availability.as_ref() {
        Some(a) if !a.part_listings.is_empty() => "ok",
        Some(_) => "catalog_only_no_listings",
        None if c.availability_error.is_some() => "catalog_only_avail_error",
        None => "catalog_only",
    }
}

const COMBINED_CSV_HEADERS: [&str; 26] = [
    "line",
    "input",
    "normalized",
    "status",
    "error",
    // government fields
    "item_name",
    "fsc",
    "niin",
    "has_parts_availability",
    "mcrl_primary_cage",
    "mcrl_primary_company",
    // availability fields
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
];

fn write_combined_csv<W: Write>(
    results: &[QueryResult<Combined>],
    writer: W,
) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_writer(writer);
    wtr.write_record(COMBINED_CSV_HEADERS)?;
    for r in results {
        let line = r.entry.line.to_string();
        let input = r.entry.raw.as_str();
        let normalized = normalized_str(&r.entry);
        match &r.outcome {
            Outcome::Ok(c) => {
                let status = combined_status(c);
                let (item_name, fsc, niin, hpa, mcrl_cage, mcrl_company) = gov_summary_fields(c);
                let err = if !c.government.faults.is_empty() {
                    join_fault_messages(
                        c.government
                            .faults
                            .iter()
                            .filter_map(|f| f.message.as_deref()),
                    )
                } else {
                    c.availability_error.clone().unwrap_or_default()
                };
                match c.availability.as_ref() {
                    Some(a) if !a.part_listings.is_empty() => {
                        for listing in &a.part_listings {
                            let items = listing
                                .parts
                                .as_ref()
                                .map(|p| p.items.as_slice())
                                .unwrap_or(&[]);
                            if items.is_empty() {
                                wtr.write_record(combined_gov_plus_listing_row(
                                    &line,
                                    input,
                                    normalized,
                                    status,
                                    &err,
                                    item_name,
                                    fsc,
                                    niin,
                                    hpa,
                                    mcrl_cage,
                                    mcrl_company,
                                    listing,
                                    None,
                                ))?;
                            } else {
                                for part in items {
                                    wtr.write_record(combined_gov_plus_listing_row(
                                        &line,
                                        input,
                                        normalized,
                                        status,
                                        &err,
                                        item_name,
                                        fsc,
                                        niin,
                                        hpa,
                                        mcrl_cage,
                                        mcrl_company,
                                        listing,
                                        Some(part),
                                    ))?;
                                }
                            }
                        }
                    }
                    _ => {
                        wtr.write_record(combined_gov_only_row(
                            &line,
                            input,
                            normalized,
                            status,
                            &err,
                            item_name,
                            fsc,
                            niin,
                            hpa,
                            mcrl_cage,
                            mcrl_company,
                        ))?;
                    }
                }
            }
            Outcome::Err(e) => {
                wtr.write_record(combined_gov_only_row(
                    &line, input, normalized, "error", e, "", "", "", "", "", "",
                ))?;
            }
            Outcome::Invalid(e) => {
                wtr.write_record(combined_gov_only_row(
                    &line, input, "", "invalid", e, "", "", "", "", "", "",
                ))?;
            }
        }
    }
    wtr.flush()?;
    Ok(())
}

fn gov_summary_fields(c: &Combined) -> (&str, &str, &str, &str, &str, &str) {
    let first = c.government.items.first();
    let item_name = first.and_then(|i| i.item_name.as_deref()).unwrap_or("");
    let fsc = first.and_then(|i| i.fsc.as_deref()).unwrap_or("");
    let niin = first.and_then(|i| i.niin.as_deref()).unwrap_or("");
    let hpa = first
        .and_then(|i| i.has_parts_availability.as_deref())
        .unwrap_or("");
    let primary_mcrl = first
        .and_then(|i| i.mcrl_data.as_ref())
        .and_then(|m| m.items.as_ref())
        .and_then(|a| a.items.first());
    let mcrl_cage = primary_mcrl.and_then(|m| m.cage.as_deref()).unwrap_or("");
    let mcrl_company = primary_mcrl
        .and_then(|m| m.company_name.as_deref())
        .unwrap_or("");
    (item_name, fsc, niin, hpa, mcrl_cage, mcrl_company)
}

#[allow(clippy::too_many_arguments)]
fn combined_gov_only_row<'a>(
    line: &'a str,
    input: &'a str,
    normalized: &'a str,
    status: &'a str,
    error: &'a str,
    item_name: &'a str,
    fsc: &'a str,
    niin: &'a str,
    hpa: &'a str,
    mcrl_cage: &'a str,
    mcrl_company: &'a str,
) -> [&'a str; 26] {
    [
        line,
        input,
        normalized,
        status,
        error,
        item_name,
        fsc,
        niin,
        hpa,
        mcrl_cage,
        mcrl_company,
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
        "",
        "",
    ]
}

#[allow(clippy::too_many_arguments)]
fn combined_gov_plus_listing_row<'a>(
    line: &'a str,
    input: &'a str,
    normalized: &'a str,
    status: &'a str,
    error: &'a str,
    item_name: &'a str,
    fsc: &'a str,
    niin: &'a str,
    hpa: &'a str,
    mcrl_cage: &'a str,
    mcrl_company: &'a str,
    listing: &'a PartListing,
    part: Option<&'a PartSearchResult>,
) -> [&'a str; 26] {
    let company = listing.company.as_ref();
    [
        line,
        input,
        normalized,
        status,
        error,
        item_name,
        fsc,
        niin,
        hpa,
        mcrl_cage,
        mcrl_company,
        company.and_then(|c| c.id.as_deref()).unwrap_or(""),
        company.and_then(|c| c.name.as_deref()).unwrap_or(""),
        company
            .and_then(|c| c.supplier_cage.as_deref())
            .unwrap_or(""),
        company
            .and_then(|c| c.accredited_vendor_level.as_deref())
            .unwrap_or(""),
        part.and_then(|p| p.part_number.as_deref()).unwrap_or(""),
        part.and_then(|p| p.alternate_part_number.as_deref())
            .unwrap_or(""),
        part.and_then(|p| p.condition_code.as_deref()).unwrap_or(""),
        part.and_then(|p| p.description.as_deref()).unwrap_or(""),
        part.and_then(|p| p.exchange_option.as_deref())
            .unwrap_or(""),
        part.and_then(|p| p.quantity.as_deref()).unwrap_or(""),
        part.and_then(|p| p.maker.as_deref()).unwrap_or(""),
        part.and_then(|p| p.model.as_deref()).unwrap_or(""),
        part.and_then(|p| p.part_entered.as_deref()).unwrap_or(""),
        part.and_then(|p| p.search_part_id.as_deref()).unwrap_or(""),
        part.and_then(|p| p.is_preferred_vendor)
            .map(|b| if b { "true" } else { "false" })
            .unwrap_or(""),
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
