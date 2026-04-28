//! PDF internal provenance analysis.
//!
//! Parses a PDF file's Info dictionary, cross-reference table structure,
//! page tree, and annotation layer to extract provenance signals:
//! producer software, creator application, creation and modification dates,
//! incremental save history, digital signatures, redaction annotations,
//! and PDF/A compliance.
//!
//! Uses the `lopdf` crate for PDF object parsing. All fields are optional
//! and degrade gracefully when parsing fails.

use lopdf::{Document, Object};
use serde::{Deserialize, Serialize};

/// Provenance signals extracted from a PDF file.
///
/// All string fields use `Option<String>` — absent metadata is `None`.
/// The `summary` field is always populated with a human-readable overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfProvenance {
    /// Producing software (e.g. "Adobe PDF Library 15.0", "Microsoft Word",
    /// "LaTeX with hyperref").
    pub producer: Option<String>,
    /// The application that created the original document (before PDF export),
    /// e.g. "Microsoft Word 2019", "Pages 12.0".
    pub creator: Option<String>,
    /// PDF creation date as recorded in the Info dictionary (raw string).
    pub creation_date: Option<String>,
    /// Last modification date (raw string). Different from creation date
    /// when the document has been edited after initial export.
    pub mod_date: Option<String>,
    /// Number of pages.
    pub page_count: u32,
    /// Whether the file contains at least one digital signature dictionary
    /// (`/Type /Sig`).
    pub has_digital_signature: bool,
    /// Whether the cross-reference table has more than one section, indicating
    /// the document was saved incrementally (i.e. at least one round of
    /// post-creation edits). Requires a linear scan of the file bytes.
    pub has_incremental_saves: bool,
    /// Whether the document contains at least one redaction annotation
    /// (`/Subtype /Redact`).
    pub has_redaction_annotations: bool,
    /// PDF version string from the file header (e.g. "1.4", "1.7", "2.0").
    pub pdf_version: String,
    /// Whether the document declares PDF/A conformance via an `OutputIntents`
    /// entry in the Document Catalog that references an ICC profile with a
    /// `GTS_PDFA1` or `GTS_PDFA` key.
    pub is_pdf_a: bool,
    /// Number of distinct font names embedded in the document.
    pub embedded_font_count: u32,
    /// Human-readable provenance summary for the UI panel.
    pub summary: String,
}

/// Analyse a PDF file and return its provenance signals.
///
/// Returns `None` only when the file cannot be opened or is not a valid PDF.
/// All individual fields degrade gracefully — a partially parseable PDF still
/// returns a `PdfProvenance` with whatever could be extracted.
pub fn analyse_pdf(path: &std::path::Path) -> Option<PdfProvenance> {
    // lopdf::Document::load returns Err for non-PDF / corrupt files.
    let doc = Document::load(path).ok()?;

    let producer = extract_info_string(&doc, b"Producer");
    let creator = extract_info_string(&doc, b"Creator");
    let creation_date = extract_info_string(&doc, b"CreationDate");
    let mod_date = extract_info_string(&doc, b"ModDate");

    let page_count = count_pages(&doc);
    let has_digital_signature = detect_signatures(&doc);
    let has_incremental_saves = detect_incremental_saves(path);
    let has_redaction_annotations = detect_redactions(&doc);
    let pdf_version = extract_version(path);
    let is_pdf_a = detect_pdf_a(&doc);
    let embedded_font_count = count_embedded_fonts(&doc);

    let summary = build_summary(
        producer.as_deref(),
        creator.as_deref(),
        page_count,
        has_digital_signature,
        has_incremental_saves,
        has_redaction_annotations,
        is_pdf_a,
        embedded_font_count,
    );

    Some(PdfProvenance {
        producer,
        creator,
        creation_date,
        mod_date,
        page_count,
        has_digital_signature,
        has_incremental_saves,
        has_redaction_annotations,
        pdf_version,
        is_pdf_a,
        embedded_font_count,
        summary,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract a string value from the document's Info dictionary.
///
/// PDF Info dictionary values are `Object::String` (bytes + encoding).
/// We decode as UTF-8 with lossy replacement.
fn extract_info_string(doc: &Document, key: &[u8]) -> Option<String> {
    let info_id = doc.trailer.get(b"Info").ok()?;
    let info_ref = match info_id {
        Object::Reference(r) => *r,
        _ => return None,
    };
    let info_dict = doc.get_object(info_ref).ok()?.as_dict().ok()?;
    let value = info_dict.get(key).ok()?;
    match value {
        Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).trim().to_string()),
        _ => None,
    }
}

/// Count the total number of pages by walking the page tree.
fn count_pages(doc: &Document) -> u32 {
    doc.get_pages().len() as u32
}

/// Search for any signature dictionary in the document objects.
///
/// A digital signature is represented as a PDF object with `/Type /Sig`.
/// We walk the object map and look for dictionaries containing this key.
fn detect_signatures(doc: &Document) -> bool {
    for (_, obj) in doc.objects.iter() {
        if let Ok(dict) = obj.as_dict() {
            if let Ok(Object::Name(type_name)) = dict.get(b"Type") {
                if type_name == b"Sig" {
                    return true;
                }
            }
        }
    }
    false
}

/// Detect incremental saves by scanning the raw file bytes for multiple
/// `startxref` markers. Each `startxref` entry in a PDF indicates one
/// cross-reference table section — one section means a single linear save,
/// more than one means incremental updates were applied.
fn detect_incremental_saves(path: &std::path::Path) -> bool {
    use std::io::Read;
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    // Scan only the last 128 KB — `startxref` entries live near the EOF.
    let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let scan_len = file_len.min(128 * 1024) as usize;
    let seek_pos = file_len.saturating_sub(scan_len as u64);
    if std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(seek_pos)).is_err() {
        return false;
    }
    let mut buf = Vec::with_capacity(scan_len);
    if file.take(scan_len as u64).read_to_end(&mut buf).is_err() {
        return false;
    }
    // Count occurrences of b"startxref" in the tail buffer.
    let mut count = 0usize;
    let needle = b"startxref";
    let mut pos = 0;
    while pos + needle.len() <= buf.len() {
        if buf[pos..pos + needle.len()] == *needle {
            count += 1;
            if count > 1 {
                return true;
            }
        }
        pos += 1;
    }
    false
}

/// Search for redaction annotations (`/Subtype /Redact`) in the document.
fn detect_redactions(doc: &Document) -> bool {
    for (_, obj) in doc.objects.iter() {
        if let Ok(dict) = obj.as_dict() {
            if let Ok(Object::Name(subtype)) = dict.get(b"Subtype") {
                if subtype == b"Redact" {
                    return true;
                }
            }
        }
    }
    false
}

/// Extract the PDF version from the file header (`%PDF-N.N`).
///
/// Reads only the first 16 bytes of the file. Returns "unknown" on failure.
fn extract_version(path: &std::path::Path) -> String {
    use std::io::Read;
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return "unknown".to_string(),
    };
    let mut header = [0u8; 16];
    if file.read_exact(&mut header).is_err() {
        return "unknown".to_string();
    }
    // Expected: b"%PDF-1.4\n" or b"%PDF-2.0\r"
    let header_str = String::from_utf8_lossy(&header);
    if let Some(rest) = header_str.strip_prefix("%PDF-") {
        let version = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect::<String>();
        if !version.is_empty() {
            return version;
        }
    }
    "unknown".to_string()
}

/// Check whether the document declares PDF/A conformance.
///
/// PDF/A conformance is declared via an `OutputIntents` array in the
/// Document Catalog. Each entry is a dictionary with an `S` key whose
/// value is one of `/GTS_PDFA1`, `/GTS_PDFA2`, `/GTS_PDFA3`, etc.
fn detect_pdf_a(doc: &Document) -> bool {
    let catalog = match doc.catalog() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let output_intents = match catalog.get(b"OutputIntents") {
        Ok(o) => o,
        Err(_) => return false,
    };
    let array = match output_intents {
        Object::Array(arr) => arr,
        _ => return false,
    };
    for item in array {
        let dict = match item {
            Object::Dictionary(d) => d,
            Object::Reference(r) => {
                if let Ok(obj) = doc.get_object(*r) {
                    if let Ok(d) = obj.as_dict() {
                        // Borrow issue: fall through to name check inline
                        if let Ok(Object::Name(s)) = d.get(b"S") {
                            if s.starts_with(b"GTS_PDFA") {
                                return true;
                            }
                        }
                    }
                }
                continue;
            }
            _ => continue,
        };
        if let Ok(Object::Name(s)) = dict.get(b"S") {
            if s.starts_with(b"GTS_PDFA") {
                return true;
            }
        }
    }
    false
}

/// Count distinct embedded font names across the document.
///
/// Walks every object looking for `/Type /Font` dictionaries and collects
/// unique `/BaseFont` names. Returns the count of distinct names found.
fn count_embedded_fonts(doc: &Document) -> u32 {
    let mut font_names = std::collections::HashSet::new();
    for (_, obj) in doc.objects.iter() {
        if let Ok(dict) = obj.as_dict() {
            if let Ok(Object::Name(type_name)) = dict.get(b"Type") {
                if type_name == b"Font" {
                    if let Ok(Object::Name(base_font)) = dict.get(b"BaseFont") {
                        font_names.insert(base_font.clone());
                    }
                }
            }
        }
    }
    font_names.len() as u32
}

/// Build a concise human-readable summary of the PDF provenance signals.
#[allow(clippy::too_many_arguments)]
fn build_summary(
    producer: Option<&str>,
    creator: Option<&str>,
    page_count: u32,
    has_digital_signature: bool,
    has_incremental_saves: bool,
    has_redaction_annotations: bool,
    is_pdf_a: bool,
    embedded_font_count: u32,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(p) = producer {
        parts.push(format!("Produced by: {p}"));
    }
    if let Some(c) = creator {
        parts.push(format!("Created with: {c}"));
    }

    if page_count > 0 {
        parts.push(format!(
            "{page_count} page{}",
            if page_count == 1 { "" } else { "s" }
        ));
    }

    if has_digital_signature {
        parts.push("Contains a digital signature.".to_string());
    }
    if has_incremental_saves {
        parts.push("Incrementally saved (multiple edit rounds detected).".to_string());
    }
    if has_redaction_annotations {
        parts.push("Contains redaction annotations.".to_string());
    }
    if is_pdf_a {
        parts.push("PDF/A archival compliance declared.".to_string());
    }
    if embedded_font_count > 0 {
        parts.push(format!("{embedded_font_count} embedded font(s)."));
    }

    if parts.is_empty() {
        "No provenance metadata available.".to_string()
    } else {
        parts.join(" ")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_summary_full() {
        let s = build_summary(
            Some("Adobe PDF Library"),
            Some("Microsoft Word"),
            5,
            true,
            true,
            false,
            true,
            3,
        );
        assert!(s.contains("Adobe PDF Library"), "producer in summary");
        assert!(s.contains("Microsoft Word"), "creator in summary");
        assert!(s.contains("5 pages"), "page count in summary");
        assert!(s.contains("digital signature"), "signature in summary");
        assert!(s.contains("Incrementally"), "incremental saves in summary");
        assert!(s.contains("PDF/A"), "pdfa in summary");
        assert!(s.contains("3 embedded font"), "fonts in summary");
    }

    #[test]
    fn build_summary_zero_pages_no_metadata() {
        // page_count=0 is skipped; with no other signals the fallback message fires.
        let s = build_summary(None, None, 0, false, false, false, false, 0);
        assert!(
            s.contains("No provenance"),
            "fallback message when all signals absent"
        );
    }

    #[test]
    fn build_summary_single_page() {
        let s = build_summary(None, None, 1, false, false, false, false, 0);
        assert!(s.contains("1 page"), "singular page");
        assert!(!s.contains("1 pages"), "should not pluralise");
    }
}
