// SPDX-License-Identifier: AGPL-3.0-or-later

//! Image metadata extraction: EXIF, XMP, ICC, and JPEG quantisation tables.
//!
//! Provides [`extract_exif`] (Tier 1 cataloguing via the `kamadak-exif`
//! crate), XMP packet parsing for AI-provenance signals such as
//! `Iptc4xmpExt:DigitalSourceType` and edit-history stacks, ICC profile
//! descriptions, EXIF thumbnail recovery, and JPEG quantisation-table
//! extraction. Also owns the known-camera-vendor list and
//! [`camera_authenticity_confidence`], which feed the EXIF anomaly detector.

use exif::{In, Reader as ExifReader, Tag, Value};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Parsed XMP (Extensible Metadata Platform) fields relevant to AI-provenance
/// detection. XMP is an XML-based metadata packet embedded in most image
/// formats (JPEG via APP1, PNG via iTXt, TIFF via tag 700). Modern AI
/// generators increasingly write provenance signals into XMP rather than
/// EXIF — most importantly `Iptc4xmpExt:DigitalSourceType`, which is the
/// ground-truth AI-origin declaration under the IPTC / C2PA / CAI framework.
///
/// All fields are `Option<String>` and default to `None` when the packet
/// is absent, malformed, or the specific field is missing. The parser is
/// deliberately tolerant — it locates the `<x:xmpmeta>` envelope by byte
/// search and extracts named fields via element / attribute matching.
/// A single edit event from the XMP edit-history stack (`xmpMM:History`).
///
/// Adobe Photoshop and other editors write a sequence of `stEvt:` entries
/// inside an `rdf:Seq` under `xmpMM:History`. Each entry records an action
/// (created, saved, converted, etc.), the software agent, an ISO 8601
/// timestamp, and optional parameters. Standard `xmpMM:History` actions
/// use XMP-defined verbs (created, saved, converted, derived, printed) —
/// individual tool names (Clone Stamp, Content-Aware Fill) are NOT part of
/// the standard vocabulary but may appear in `stEvt:parameters` or in
/// custom pipelines.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmpHistoryEvent {
    /// The action performed: "created", "saved", "converted", etc.
    pub action: String,
    /// The software agent that performed the action (e.g. "Adobe Photoshop 25.0 (Macintosh)").
    pub software_agent: String,
    /// ISO 8601 timestamp of the action, if present.
    pub when: Option<String>,
    /// Additional parameters (e.g. "converted from image/jpeg to image/jpeg",
    /// or occasionally tool-level detail like "content-aware fill").
    pub parameters: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XmpMetadata {
    /// `Iptc4xmpExt:DigitalSourceType` — the IPTC-standard AI-provenance
    /// declaration. Values of interest:
    /// - `trainedAlgorithmicMedia` (AI-generated, e.g. Firefly, DALL-E)
    /// - `compositeWithTrainedAlgorithmicMedia` (AI components in a real photo)
    /// - `algorithmicMedia` (algorithmically composed, non-trained)
    /// - `digitalCapture` (camera capture — benign authentic value)
    ///
    /// Stored as the short local name (without the IPTC namespace URL prefix)
    /// e.g. `trainedAlgorithmicMedia`.
    pub digital_source_type: Option<String>,
    /// `xmp:CreatorTool` — name and version of the software that produced
    /// the image, e.g. "Adobe Photoshop 25.0", "Stable Diffusion 1.5",
    /// "DALL-E 3", "Midjourney 6".
    pub creator_tool: Option<String>,
    /// `photoshop:Credit` — attribution string.
    pub credit: Option<String>,
    /// `dc:creator` — author claim. When the source XMP stores this as an
    /// `rdf:Seq` / `rdf:Bag`, the first `<rdf:li>` value is captured.
    pub creator: Option<String>,
    /// Parsed `xmpMM:History` edit-history stack. Each entry records one
    /// action (created, saved, converted, etc.) with software agent and
    /// timestamp. Empty when no history block is present.
    #[serde(default)]
    pub history: Vec<XmpHistoryEvent>,
}

/// Quantisation tables extracted from a JPEG file's DQT markers.
///
/// Exposes the raw 8×8 luminance and chrominance tables, an estimated IJG
/// quality factor, and an optional match against a small built-in database
/// of known software signatures.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JpegQuantTables {
    /// Luminance Q-table (64 values, row-major 8×8). Present when a table
    /// with destination ID 0 was found in the DQT markers.
    pub luminance: Option<Vec<u16>>,
    /// Chrominance Q-table (64 values, row-major 8×8). Present when a table
    /// with destination ID 1 was found in the DQT markers.
    pub chrominance: Option<Vec<u16>>,
    /// Estimated IJG-equivalent quality factor (1–100) derived by comparing
    /// the luminance table against the standard IJG baseline table.
    /// `None` when the luminance table is absent.
    pub estimated_quality: Option<u32>,
    /// Name of the known encoder/software matched in the built-in database
    /// (e.g. "Photoshop Save for Web Q80", "Google Photos"). `None` when no
    /// match was found.
    pub known_source: Option<String>,
}

/// Extracted metadata from an image file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
    /// Camera make (e.g. "Canon")
    pub camera_make: Option<String>,
    /// Camera model (e.g. "EOS R5")
    pub camera_model: Option<String>,
    /// Software used (e.g. "Adobe Photoshop 25.0")
    pub software: Option<String>,
    /// Original datetime as stored in EXIF
    pub datetime_original: Option<String>,
    /// DateTime the file was last modified
    pub datetime_modified: Option<String>,
    /// Image width in pixels (from EXIF, not decoded image)
    pub exif_width: Option<u32>,
    /// Image height in pixels
    pub exif_height: Option<u32>,
    /// Colour space description
    pub color_space: Option<String>,
    /// GPS latitude (decimal degrees, north positive)
    pub gps_latitude: Option<f64>,
    /// GPS longitude (decimal degrees, east positive)
    pub gps_longitude: Option<f64>,
    /// ISO speed
    pub iso: Option<u32>,
    /// Focal length in mm
    pub focal_length: Option<String>,
    /// Exposure time (e.g. "1/250")
    pub exposure_time: Option<String>,
    /// F-number (e.g. "f/2.8")
    pub f_number: Option<String>,
    /// Copyright string
    pub copyright: Option<String>,
    /// Artist / creator
    pub artist: Option<String>,
    /// Image description
    pub description: Option<String>,
    /// EXIF orientation value (1-8)
    pub orientation: Option<u16>,
    /// Whether the file contains a non-empty MakerNote tag.
    /// MakerNotes are vendor-proprietary binary blobs that AI image generators
    /// virtually never produce. A present, non-empty MakerNote whose camera_make
    /// matches a known vendor is a strong positive authenticity signal.
    pub has_maker_note: bool,
    /// MakerNote byte length (0 when absent). Provides confidence — most genuine
    /// camera MakerNotes are 1–50 KB; a 4-byte placeholder is suspicious.
    pub maker_note_length: usize,
    /// Parsed XMP packet fields relevant to AI-provenance detection. Defaults
    /// to an all-`None` struct when the file has no XMP packet, the packet is
    /// malformed, or none of the tracked fields are present.
    #[serde(default)]
    pub xmp: XmpMetadata,
    /// ICC colour profile description string extracted from the embedded ICC
    /// profile (e.g. "sRGB IEC61966-2.1", "Display P3", "Canon EOS R5").
    /// `None` when no ICC profile is embedded or it could not be parsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icc_profile_description: Option<String>,
    /// JPEG quantisation tables extracted from DQT markers.
    /// Only populated for JPEG files (`image/jpeg`). `None` for all other
    /// formats or when DQT markers could not be located.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jpeg_quant_tables: Option<JpegQuantTables>,
}

/// Extract EXIF metadata from an image file.
///
/// Returns `None` if the file has no EXIF data or is not a supported format
/// (JPEG, TIFF, WebP, HEIF are supported by kamadak-exif).
pub fn extract_exif(path: &Path) -> Option<ImageMetadata> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let exif = ExifReader::new()
        .read_from_container(&mut std::io::BufReader::new(reader))
        .ok()?;

    // XMP is a separate packet from EXIF and may exist even when EXIF is thin.
    // Read it from the raw file bytes; failures are non-fatal.
    let xmp = extract_xmp(path).unwrap_or_default();

    let get_str = |tag: Tag| -> Option<String> {
        exif.get_field(tag, In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string())
    };

    let get_u32 = |tag: Tag| -> Option<u32> {
        exif.get_field(tag, In::PRIMARY)
            .and_then(|f| match &f.value {
                Value::Long(v) => v.first().copied(),
                Value::Short(v) => v.first().map(|&x| x as u32),
                _ => f.display_value().to_string().parse().ok(),
            })
    };

    let get_u16 = |tag: Tag| -> Option<u16> {
        exif.get_field(tag, In::PRIMARY)
            .and_then(|f| match &f.value {
                Value::Short(v) => v.first().copied(),
                _ => f.display_value().to_string().parse().ok(),
            })
    };

    // GPS coordinate extraction
    let gps_latitude = extract_gps_coord(&exif, Tag::GPSLatitude, Tag::GPSLatitudeRef);
    let gps_longitude = extract_gps_coord(&exif, Tag::GPSLongitude, Tag::GPSLongitudeRef);

    // MakerNote presence and length
    let (has_maker_note, maker_note_length) = exif
        .get_field(Tag::MakerNote, In::PRIMARY)
        .map(|f| match &f.value {
            Value::Undefined(bytes, _) => (!bytes.is_empty(), bytes.len()),
            _ => (false, 0),
        })
        .unwrap_or((false, 0));

    // Extract ICC profile description from the raw file bytes.
    // This is format-independent — the same parser handles JPEG APP2, PNG
    // iCCP, and TIFF tag 34675 (InterColorProfile).
    let icc_profile_description = extract_icc_profile_description(path);

    // Extract JPEG quantisation tables. Only attempted for JPEG files.
    let jpeg_quant_tables = extract_jpeg_quant_tables(path);

    Some(ImageMetadata {
        camera_make: get_str(Tag::Make).map(|s| s.trim_matches('"').to_string()),
        camera_model: get_str(Tag::Model).map(|s| s.trim_matches('"').to_string()),
        software: get_str(Tag::Software).map(|s| s.trim_matches('"').to_string()),
        datetime_original: get_str(Tag::DateTimeOriginal).map(|s| s.trim_matches('"').to_string()),
        datetime_modified: get_str(Tag::DateTime).map(|s| s.trim_matches('"').to_string()),
        exif_width: get_u32(Tag::PixelXDimension).or_else(|| get_u32(Tag::ImageWidth)),
        exif_height: get_u32(Tag::PixelYDimension).or_else(|| get_u32(Tag::ImageLength)),
        color_space: get_str(Tag::ColorSpace),
        gps_latitude,
        gps_longitude,
        iso: get_u32(Tag::PhotographicSensitivity),
        focal_length: get_str(Tag::FocalLength),
        exposure_time: get_str(Tag::ExposureTime),
        f_number: get_str(Tag::FNumber),
        copyright: get_str(Tag::Copyright).map(|s| s.trim_matches('"').to_string()),
        artist: get_str(Tag::Artist).map(|s| s.trim_matches('"').to_string()),
        description: get_str(Tag::ImageDescription).map(|s| s.trim_matches('"').to_string()),
        orientation: get_u16(Tag::Orientation),
        has_maker_note,
        maker_note_length,
        xmp,
        icc_profile_description,
        jpeg_quant_tables,
    })
}

/// Maximum number of bytes we will read from the head of a file when hunting
/// for an XMP packet. XMP is conventionally placed in the first APP1 segment
/// (JPEG) or the first iTXt chunk (PNG) near the file head. 64 KB is
/// sufficient to cover the vast majority of real-world XMP placements; the
/// previous 2 MB cap caused unnecessary buffering on every import.
const XMP_SCAN_LIMIT: usize = 64 * 1024;

/// Locate and parse an XMP packet from a file.
///
/// Reads up to [`XMP_SCAN_LIMIT`] bytes from the head of the file, searches
/// for an `<x:xmpmeta ...>` ... `</x:xmpmeta>` envelope, and extracts the
/// four AI-provenance fields tracked by [`XmpMetadata`]. Returns `None` if
/// the file cannot be opened or no XMP envelope is present; returns
/// `Some(XmpMetadata::default())` if an envelope is present but contains
/// none of the tracked fields (so callers can still distinguish "no packet"
/// from "packet present, no useful fields").
pub fn extract_xmp(path: &Path) -> Option<XmpMetadata> {
    let file = File::open(path).ok()?;
    let mut buf = Vec::with_capacity(64 * 1024);
    file.take(XMP_SCAN_LIMIT as u64)
        .read_to_end(&mut buf)
        .ok()?;
    parse_xmp_packet(&buf)
}

/// Parse an XMP packet out of an arbitrary byte slice. Exposed for unit
/// testing with synthetic payloads.
pub fn parse_xmp_packet(bytes: &[u8]) -> Option<XmpMetadata> {
    // `<x:xmpmeta` is the canonical envelope opener written by Adobe's XMP
    // toolkit and every library that follows the XMP specification. We do
    // not try to handle the rarer `<?xpacket ...?>` standalone form without
    // the envelope — those payloads are handled correctly because `xpacket`
    // normally wraps an `x:xmpmeta` element too.
    let open = find_subsequence(bytes, b"<x:xmpmeta")?;
    let close_marker = b"</x:xmpmeta>";
    let close = find_subsequence(&bytes[open..], close_marker)?;
    let end = open + close + close_marker.len();
    // XMP is guaranteed ASCII-safe for the element and attribute names we
    // care about; lossy UTF-8 decoding keeps us robust against stray bytes.
    let packet = String::from_utf8_lossy(&bytes[open..end]);
    let packet = packet.as_ref();

    let digital_source_type = extract_digital_source_type(packet);
    let creator_tool = extract_xmp_field(packet, "xmp:CreatorTool")
        .or_else(|| extract_xmp_field(packet, "tiff:Software"));
    let credit = extract_xmp_field(packet, "photoshop:Credit");
    let creator = extract_dc_creator(packet);
    let history = extract_xmp_history(packet);

    Some(XmpMetadata {
        digital_source_type,
        creator_tool,
        credit,
        creator,
        history,
    })
}

/// Extract a single-valued XMP field. Handles both the element form
/// `<ns:Name>value</ns:Name>` and the attribute form `ns:Name="value"`
/// that Adobe's toolkit emits when the value is scalar.
fn extract_xmp_field(packet: &str, qualified_name: &str) -> Option<String> {
    // Attribute form first — it is the more common shape for short scalar
    // fields like `xmp:CreatorTool` when the emitter is Adobe / Lightroom.
    let attr_key = format!("{qualified_name}=\"");
    if let Some(start) = packet.find(&attr_key) {
        let after = &packet[start + attr_key.len()..];
        if let Some(end) = after.find('"') {
            let value = decode_xml_entities(after[..end].trim());
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    // Element form — `<ns:Name ...>value</ns:Name>`. We tolerate attributes
    // on the opening tag by scanning forward to the first `>`.
    let open_tag = format!("<{qualified_name}");
    if let Some(start) = packet.find(&open_tag) {
        let after_open = &packet[start + open_tag.len()..];
        if let Some(gt) = after_open.find('>') {
            let value_start = gt + 1;
            let close_tag = format!("</{qualified_name}>");
            if let Some(close) = after_open[value_start..].find(&close_tag) {
                let raw = &after_open[value_start..value_start + close];
                let value = decode_xml_entities(raw.trim());
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// Extract `Iptc4xmpExt:DigitalSourceType`. The value is stored as an IRI
/// (e.g. `http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia`)
/// on either a `<rdf:value>` child, a direct element, or a scalar attribute.
/// We return just the short local-name suffix.
fn extract_digital_source_type(packet: &str) -> Option<String> {
    let raw = extract_xmp_field(packet, "Iptc4xmpExt:DigitalSourceType")?;
    // Strip any IRI prefix — the tracked values live in the last path segment.
    let short = raw.rsplit('/').next().unwrap_or(&raw).trim().to_string();
    if short.is_empty() {
        None
    } else {
        Some(short)
    }
}

/// Extract `dc:creator`. Stored as an `rdf:Seq` / `rdf:Bag` of `rdf:li`
/// entries — we return the first non-empty list item.
fn extract_dc_creator(packet: &str) -> Option<String> {
    // Simple scalar form first (rare for dc:creator but possible).
    if let Some(v) = extract_xmp_field(packet, "dc:creator") {
        // If the scalar is a container literal like "<rdf:Seq>...</rdf:Seq>",
        // fall through to the list handler below.
        if !v.contains("rdf:") && !v.contains('<') {
            return Some(v);
        }
    }
    // List form — pull the first <rdf:li>...</rdf:li> that appears *after*
    // the dc:creator opener, to avoid matching an unrelated list elsewhere.
    let start = packet.find("<dc:creator")?;
    let tail = &packet[start..];
    let end = tail.find("</dc:creator>").unwrap_or(tail.len());
    let section = &tail[..end];
    let li_open = section.find("<rdf:li")?;
    let after_open = &section[li_open..];
    let gt = after_open.find('>')?;
    let value_start = gt + 1;
    let li_close = after_open[value_start..].find("</rdf:li>")?;
    let raw = &after_open[value_start..value_start + li_close];
    let value = decode_xml_entities(raw.trim());
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Extract the `xmpMM:History` edit-history stack as a list of
/// [`XmpHistoryEvent`] entries.
///
/// The history block is an `rdf:Seq` of `rdf:li` entries, each carrying
/// `stEvt:action`, `stEvt:softwareAgent`, `stEvt:when`, `stEvt:parameters`,
/// and optionally `stEvt:changed`. Fields appear as either XML attributes
/// on the `<rdf:li>` element or as child elements — both forms are handled.
fn extract_xmp_history(packet: &str) -> Vec<XmpHistoryEvent> {
    // Locate the xmpMM:History block.
    let history_start = match packet.find("<xmpMM:History") {
        Some(pos) => pos,
        None => return Vec::new(),
    };
    let history_end = match packet[history_start..].find("</xmpMM:History>") {
        Some(pos) => history_start + pos,
        None => {
            // Try self-closing or truncated — no usable data.
            return Vec::new();
        }
    };
    let section = &packet[history_start..history_end];

    let mut events = Vec::new();
    let mut search_from = 0;

    while let Some(li_start) = section[search_from..].find("<rdf:li") {
        let abs_li_start = search_from + li_start;

        // Find the end of this rdf:li — either self-closing or paired close.
        let after_li = &section[abs_li_start..];
        let (li_content, li_end_offset) = if let Some(sc) = find_self_close_or_close(after_li) {
            sc
        } else {
            break;
        };

        let action = extract_stevt_field(li_content, "stEvt:action");
        let software_agent = extract_stevt_field(li_content, "stEvt:softwareAgent");
        let when = extract_stevt_field(li_content, "stEvt:when");
        let parameters = extract_stevt_field(li_content, "stEvt:parameters");

        events.push(XmpHistoryEvent {
            action: action.unwrap_or_default(),
            software_agent: software_agent.unwrap_or_default(),
            when,
            parameters,
        });

        search_from = abs_li_start + li_end_offset;
    }

    events
}

/// Find the extent of an `<rdf:li ...>` element — handles both self-closing
/// (`/>`) and paired (`</rdf:li>`) forms. Returns `(content_slice, end_offset)`
/// where `content_slice` is everything from the opening `<rdf:li` to the
/// closing delimiter, and `end_offset` is the byte position just past the
/// closing delimiter relative to the input.
fn find_self_close_or_close(s: &str) -> Option<(&str, usize)> {
    // Find the first `>` or `/>` after the tag name.
    let mut pos = 0;
    while pos < s.len() {
        if s[pos..].starts_with("/>") {
            // Self-closing: content is everything up to and including `/>`.
            let end = pos + 2;
            return Some((&s[..end], end));
        } else if s[pos..].starts_with('>') {
            // Paired element — look for `</rdf:li>`.
            if let Some(close) = s[pos..].find("</rdf:li>") {
                let end = pos + close + "</rdf:li>".len();
                return Some((&s[..end], end));
            } else {
                // Malformed — no closing tag.
                return None;
            }
        }
        pos += 1;
    }
    None
}

/// Extract a single `stEvt:*` field from an `rdf:li` element's content.
/// Handles both the attribute form (`stEvt:action="created"`) and the
/// child-element form (`<stEvt:action>created</stEvt:action>`).
fn extract_stevt_field(li_content: &str, qualified_name: &str) -> Option<String> {
    // Attribute form: stEvt:action="value"
    let attr_key = format!("{qualified_name}=\"");
    if let Some(start) = li_content.find(&attr_key) {
        let after = &li_content[start + attr_key.len()..];
        if let Some(end) = after.find('"') {
            let value = decode_xml_entities(after[..end].trim());
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    // Child-element form: <stEvt:action>value</stEvt:action>
    let open_tag = format!("<{qualified_name}");
    if let Some(start) = li_content.find(&open_tag) {
        let after_open = &li_content[start + open_tag.len()..];
        if let Some(gt) = after_open.find('>') {
            let value_start = gt + 1;
            let close_tag = format!("</{qualified_name}>");
            if let Some(close) = after_open[value_start..].find(&close_tag) {
                let raw = &after_open[value_start..value_start + close];
                let value = decode_xml_entities(raw.trim());
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// Minimal XML entity decoder for the five predefined entities. XMP fields
/// rarely carry numeric character references for the values we care about,
/// so full entity expansion is not required.
fn decode_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Byte-level substring search. Avoids pulling in an additional crate.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Known camera vendors that produce parseable MakerNotes.
/// AI image generators do not synthesise these — a clean MakerNote with
/// a matching `Make` tag is a high-confidence authenticity signal.
const KNOWN_CAMERA_VENDORS: &[&str] = &[
    "apple",      // iPhone (Deep Fusion / Smart HDR pipeline)
    "google",     // Pixel (HDR+ / Computational Photo)
    "samsung",    // Galaxy S/A series
    "sony",       // ILCE, DSC, Xperia
    "canon",      // EOS, PowerShot
    "nikon",      // Z, D, Coolpix
    "fujifilm",   // X-series
    "panasonic",  // Lumix
    "olympus",    // OM-D, PEN
    "om digital", // OM-1 successor brand
    "leica",
    "sigma", // fp series
    "ricoh", // GR
    "pentax",
    "hasselblad",
    "phase one",
    "dji", // drones — Mavic, Mini, Air, Phantom
    "gopro",
    "insta360",
    "huawei",
    "xiaomi",
    "oppo",
    "vivo",
    "oneplus",
    "realme",
    "tecno",   // mid-range, common in Global Majority markets
    "infinix", // mid-range, common in Global Majority markets
    "itel",
    "honor",
    "asus",
    "motorola",
    "nokia",
    "lg electronics",
    "blackberry",
    "tcl",
    "lenovo",
];

/// Whether the supplied EXIF `Make` string matches a vendor in
/// `KNOWN_CAMERA_VENDORS`. Case-insensitive substring match. Used as a
/// softer positive-authenticity signal when MakerNote has been stripped
/// (common on photos that have been through social-platform re-encoding
/// or older email forwarding paths) but the EXIF block otherwise looks
/// camera-shaped. AI generators do not typically populate plausible
/// Make+Model strings; when they do, the EXIF injection-detection suite
/// (`check_templated_timestamps`, `check_integer_degree_gps`,
/// `check_pipeline_library_software`) catches the templating.
pub fn is_known_camera_vendor(make: Option<&str>) -> bool {
    let Some(m) = make else { return false };
    let lower = m.to_lowercase();
    KNOWN_CAMERA_VENDORS.iter().any(|v| lower.contains(v))
}

/// Determine whether the metadata indicates a genuine camera-origin image
/// based on MakerNote presence + vendor match. Returns a confidence score
/// in [0.0, 1.0] where higher = more confident the image came from a real camera.
///
/// This is a positive authenticity signal designed to mitigate false positives
/// on computational photography output (Pixel HDR+, iPhone Deep Fusion, etc.)
/// which the deepfake classifier confuses with AI-generated content.
pub fn camera_authenticity_confidence(meta: &ImageMetadata) -> f64 {
    if !meta.has_maker_note || meta.maker_note_length < 16 {
        // No MakerNote, or trivially small (suspicious — could be a placeholder)
        return 0.0;
    }

    let make_lower = meta
        .camera_make
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    if make_lower.is_empty() {
        // MakerNote present but no Make tag — unusual, low confidence
        return 0.3;
    }

    let vendor_match = KNOWN_CAMERA_VENDORS.iter().any(|v| make_lower.contains(v));

    if !vendor_match {
        // MakerNote present but vendor not recognised — moderate confidence,
        // could be a less-common manufacturer not in the table
        return 0.4;
    }

    // Strong signal: vendor matches AND MakerNote is substantial
    if meta.maker_note_length >= 256 {
        // Typical genuine MakerNote (1-50 KB range)
        1.0
    } else {
        // Vendor matches but MakerNote is unusually small
        0.7
    }
}

/// Convert EXIF GPS rational values + reference (N/S or E/W) to decimal degrees.
fn extract_gps_coord(exif: &exif::Exif, coord_tag: Tag, ref_tag: Tag) -> Option<f64> {
    let field = exif.get_field(coord_tag, In::PRIMARY)?;
    let ref_field = exif.get_field(ref_tag, In::PRIMARY)?;

    let rationals = match &field.value {
        Value::Rational(v) if v.len() >= 3 => v,
        _ => return None,
    };

    let degrees = rationals[0].to_f64();
    let minutes = rationals[1].to_f64();
    let seconds = rationals[2].to_f64();
    let decimal = degrees + minutes / 60.0 + seconds / 3600.0;

    let ref_str = ref_field.display_value().to_string();
    let sign = if ref_str.contains('S') || ref_str.contains('W') {
        -1.0
    } else {
        1.0
    };

    Some(decimal * sign)
}

/// Get image dimensions by actually decoding the image header.
/// This is a fallback when EXIF does not contain dimension tags.
pub fn get_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}

/// Extract the embedded EXIF thumbnail from an image file.
///
/// Reads `JPEGInterchangeFormat` (byte offset into the file) and
/// `JPEGInterchangeFormatLength` (byte count) from the THUMBNAIL IFD,
/// then seeks to that position in the file and returns the raw JPEG bytes.
///
/// Returns `None` if:
/// - The file has no EXIF data
/// - The THUMBNAIL IFD does not contain both required tags
/// - The offset/length values are zero or would read past end-of-file
/// - Any I/O error occurs
pub fn extract_exif_thumbnail(path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let exif = {
        let mut buf_reader = BufReader::new(&file);
        ExifReader::new()
            .read_from_container(&mut buf_reader)
            .ok()?
    };

    // Both tags must be present in the THUMBNAIL IFD (In::THUMBNAIL = IFD1).
    let offset_field = exif.get_field(Tag::JPEGInterchangeFormat, In::THUMBNAIL)?;
    let length_field = exif.get_field(Tag::JPEGInterchangeFormatLength, In::THUMBNAIL)?;

    let offset = match &offset_field.value {
        Value::Long(v) => *v.first()? as u64,
        _ => return None,
    };
    let length = match &length_field.value {
        Value::Long(v) => *v.first()? as usize,
        _ => return None,
    };

    if offset == 0 || length == 0 {
        return None;
    }

    // Seek to the thumbnail data and read exactly `length` bytes.
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut thumb_bytes = vec![0u8; length];
    file.read_exact(&mut thumb_bytes).ok()?;

    Some(thumb_bytes)
}

// ── ICC colour profile extraction ────────────────────────────────────────────

/// Maximum bytes to scan when hunting for an ICC profile chunk.
/// Real-world ICC profiles in JPEG/PNG files appear within the first few
/// kilobytes (APP2 marker in JPEG, iCCP chunk near the PNG IHDR). 512 KB is
/// a generous cap that catches all legitimate placements while cutting the
/// previous 4 MB maximum buffer cost by 8×.
const ICC_SCAN_LIMIT: usize = 512 * 1024;

/// Extract the human-readable description string from an embedded ICC profile.
///
/// Supports three container formats:
/// - JPEG: APP2 marker (`0xFF 0xE2`) with `ICC_PROFILE\0` identifier
/// - PNG: `iCCP` chunk
/// - TIFF/other: raw ICC data scanned by searching for the `acsp` magic bytes
///
/// The ICC profile header is a 128-byte fixed-size block. The profile
/// description is stored as a tagged element in the tag table that starts at
/// byte 128. We locate the `desc` tag (or `mluc` tag for v4 multi-locale
/// profiles), read the offset/length from the tag entry, and extract the
/// ASCII or UTF-16BE text.
///
/// Returns `None` when no ICC profile is found or it cannot be parsed.
pub fn extract_icc_profile_description(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut buf = Vec::with_capacity(64 * 1024);
    file.take(ICC_SCAN_LIMIT as u64)
        .read_to_end(&mut buf)
        .ok()?;

    // Try to locate the ICC profile payload in the file bytes.
    let icc_data = locate_icc_payload(&buf)?;
    parse_icc_description(icc_data)
}

/// Locate the raw ICC profile bytes within the file's byte buffer.
///
/// JPEG: APP2 marker + "ICC_PROFILE\0" prefix (can be split across multiple
/// APP2 segments for large profiles — we only read segment 1/first chunk
/// which always contains the header and tag table).
///
/// PNG: `iCCP` chunk (chunk type bytes 0x69 0x43 0x43 0x50), followed by
/// null-terminated profile name, compression method byte (0 = deflate),
/// then compressed ICC data. We skip the compressed case and fall through
/// to the generic search.
///
/// Generic: search for `acsp` magic bytes (bytes 36–39 of every ICC profile
/// header), then back up 36 bytes to the start of the header.
fn locate_icc_payload(buf: &[u8]) -> Option<&[u8]> {
    // JPEG APP2: 0xFF 0xE2 followed by 2-byte length, then "ICC_PROFILE\0"
    let icc_marker = b"ICC_PROFILE\x00";
    let mut pos = 0;
    while pos + 4 < buf.len() {
        if buf[pos] == 0xFF && buf[pos + 1] == 0xE2 {
            let seg_len = u16::from_be_bytes([buf[pos + 2], buf[pos + 3]]) as usize;
            let data_start = pos + 4;
            if data_start + icc_marker.len() <= buf.len()
                && &buf[data_start..data_start + icc_marker.len()] == icc_marker
            {
                // Skip the 12-byte identifier ("ICC_PROFILE\0" + 1-byte sequence + 1-byte total)
                let icc_start = data_start + 14;
                let icc_end = (pos + 2 + seg_len).min(buf.len());
                if icc_end > icc_start {
                    return Some(&buf[icc_start..icc_end]);
                }
            }
            pos += 2 + seg_len;
        } else {
            pos += 1;
        }
    }

    // PNG iCCP chunk: look for the 4-byte chunk type then skip name+method.
    if let Some(iccp_pos) = find_subsequence(buf, b"iCCP") {
        // iccp_pos points at 'i' of chunk type; data follows immediately.
        // PNG chunk layout: 4-byte length | 4-byte type | data | 4-byte CRC
        // The length field is 4 bytes *before* the type field.
        let data_start = iccp_pos + 4; // skip chunk type
                                       // Skip null-terminated profile name then 1-byte compression method.
        if let Some(null_pos) = buf[data_start..].iter().position(|&b| b == 0) {
            let compressed_start = data_start + null_pos + 1 + 1; // null + method byte
                                                                  // The data is zlib-compressed; skip decompression and fall through
                                                                  // to the generic `acsp` search which will still find raw profiles.
            let _ = compressed_start; // suppress unused warning
        }
    }

    // Generic fallback: search for `acsp` at byte 36 of any ICC profile block.
    if let Some(acsp_pos) = find_subsequence(buf, b"acsp") {
        if acsp_pos >= 36 {
            return Some(&buf[acsp_pos - 36..]);
        }
    }

    None
}

/// Parse the `desc` or `mluc` tag from a raw ICC profile byte slice and
/// return a UTF-8 description string.
///
/// ICC profile layout (v2 and v4):
/// - Bytes 0–3: profile size (big-endian u32)
/// - Bytes 36–39: `acsp` signature (validated by caller via `locate_icc_payload`)
/// - Bytes 128+: tag table — 4-byte count, then N × 12-byte entries
///   (4-byte tag signature, 4-byte offset, 4-byte size)
///
/// The `desc` tag (v2) contains a `mluc` or `desc` structure.
/// For simplicity we handle: `mluc` (most v4) and `desc` text (v2).
fn parse_icc_description(data: &[u8]) -> Option<String> {
    if data.len() < 132 {
        return None;
    }

    // Validate `acsp` signature at offset 36.
    if &data[36..40] != b"acsp" {
        return None;
    }

    // Tag table count at offset 128.
    let tag_count = u32::from_be_bytes(data[128..132].try_into().ok()?) as usize;
    let tag_table_start = 132_usize;

    for i in 0..tag_count {
        let entry_start = tag_table_start + i * 12;
        if entry_start + 12 > data.len() {
            break;
        }
        let tag_sig = &data[entry_start..entry_start + 4];
        if tag_sig != b"desc" {
            continue;
        }
        let offset =
            u32::from_be_bytes(data[entry_start + 4..entry_start + 8].try_into().ok()?) as usize;
        let size =
            u32::from_be_bytes(data[entry_start + 8..entry_start + 12].try_into().ok()?) as usize;

        if offset + size > data.len() || size < 12 {
            break;
        }

        let tag_data = &data[offset..offset + size];
        let type_sig = &tag_data[0..4];

        if type_sig == b"mluc" {
            // Multi-locale Unicode (ICC v4): 8-byte header then records.
            // Each record: 2-byte language, 2-byte country, 4-byte length, 4-byte offset.
            if tag_data.len() < 16 {
                break;
            }
            let record_count = u32::from_be_bytes(tag_data[8..12].try_into().ok()?) as usize;
            let record_size = u32::from_be_bytes(tag_data[12..16].try_into().ok()?) as usize;
            if record_count == 0 || record_size < 12 {
                break;
            }
            let rec = &tag_data[16..];
            if rec.len() < 12 {
                break;
            }
            let str_len = u32::from_be_bytes(rec[4..8].try_into().ok()?) as usize;
            let str_off = u32::from_be_bytes(rec[8..12].try_into().ok()?) as usize;
            if str_off + str_len > tag_data.len() || str_len < 2 {
                break;
            }
            // UTF-16BE bytes
            let utf16_bytes = &tag_data[str_off..str_off + str_len];
            let utf16_units: Vec<u16> = utf16_bytes
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            let s = String::from_utf16_lossy(&utf16_units);
            let trimmed = s.trim_matches('\0').trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        } else if type_sig == b"desc" {
            // ICC v2 `desc` type: 4-byte type sig, 4-byte reserved, 4-byte ASCII length,
            // then null-terminated ASCII string.
            if tag_data.len() < 12 {
                break;
            }
            let ascii_len = u32::from_be_bytes(tag_data[8..12].try_into().ok()?) as usize;
            if ascii_len == 0 || 12 + ascii_len > tag_data.len() {
                break;
            }
            let ascii_bytes = &tag_data[12..12 + ascii_len];
            let s = String::from_utf8_lossy(ascii_bytes)
                .trim_matches('\0')
                .trim()
                .to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
        break;
    }

    None
}

// ── JPEG quantisation table extraction ───────────────────────────────────────

/// IJG standard luminance quantisation table at quality 50 (the base table
/// that IJG scales linearly).  Used to estimate the quality factor by
/// comparing each coefficient and inferring the scale factor.
///
/// Reference: Independent JPEG Group `jctrans.c` / `jdct.h` default tables.
const IJG_LUMA_Q50: [u16; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113,
    92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
];

/// Small built-in Q-table fingerprint database.
///
/// Entries: (luminance_q50_match_description, luma_table_sample_signature).
/// The signature is the first four DC/low-frequency coefficients of the
/// luminance table at that quality level — enough to distinguish common
/// encoders without storing 64 coefficients per entry.
///
/// Keyed on (luma[0], luma[1], luma[2], luma[5]) — a 4-coefficient fingerprint
/// that is stable across most encoder variants.
const QTABLE_SIGNATURES: &[([u16; 4], &str)] = &[
    // IJG standard at common quality levels
    ([2, 1, 1, 2], "IJG standard Q95"),
    ([2, 2, 2, 3], "IJG standard Q90"),
    ([3, 2, 2, 3], "IJG standard Q85"),
    ([4, 3, 3, 4], "IJG standard Q80"),
    ([5, 3, 3, 5], "IJG standard Q75"),
    ([8, 6, 6, 8], "IJG standard Q60"),
    ([16, 11, 10, 40], "IJG standard Q50 (baseline)"),
    // Photoshop Save for Web / Export As
    ([2, 1, 1, 2], "Photoshop Save for Web Q92"),
    ([4, 3, 2, 3], "Photoshop Save for Web Q80"),
    ([6, 4, 4, 5], "Photoshop Save for Web Q70"),
    ([8, 6, 5, 8], "Photoshop Save for Web Q60"),
    // Google Photos / Pixel Camera (aggressive chroma subsampling + custom table)
    (
        [1, 1, 1, 1],
        "Google Photos / Pixel Camera (very high quality)",
    ),
    ([2, 1, 1, 2], "Google Photos Q85-95 range"),
    // Apple HEIC-to-JPEG transcoding
    ([2, 1, 1, 2], "Apple HEIC-to-JPEG export (high quality)"),
    // ImageMagick default (uses IJG tables but often at Q92)
    ([2, 1, 1, 2], "ImageMagick default Q92"),
];

/// Extract JPEG quantisation tables from DQT markers in a file.
///
/// Parses the JPEG bitstream looking for `0xFF 0xDB` (DQT) marker segments.
/// Each DQT segment may contain one or more tables; table destination ID 0 is
/// conventionally luminance, ID 1 is chrominance.
///
/// Returns `None` for non-JPEG files (no `0xFF 0xD8` SOI marker) or when
/// DQT markers cannot be located.
pub fn extract_jpeg_quant_tables(path: &Path) -> Option<JpegQuantTables> {
    let file = std::fs::File::open(path).ok()?;
    // 128 KB is more than enough to find DQT markers (they appear before the
    // SOS marker in any compliant JPEG, typically within the first 4 KB).
    let mut buf = Vec::with_capacity(128 * 1024);
    file.take(128 * 1024).read_to_end(&mut buf).ok()?;

    // Verify JPEG SOI marker.
    if buf.len() < 2 || buf[0] != 0xFF || buf[1] != 0xD8 {
        return None;
    }

    let mut luminance: Option<Vec<u16>> = None;
    let mut chrominance: Option<Vec<u16>> = None;
    let mut pos = 2_usize;

    while pos + 3 < buf.len() {
        // JPEG markers start with 0xFF.
        if buf[pos] != 0xFF {
            break;
        }
        let marker = buf[pos + 1];

        // SOI (0xD8) and EOI (0xD9) have no length field.
        if marker == 0xD8 || marker == 0xD9 {
            pos += 2;
            continue;
        }
        // SOS (0xDA) — entropy-coded data follows; stop scanning.
        if marker == 0xDA {
            break;
        }

        if pos + 4 > buf.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([buf[pos + 2], buf[pos + 3]]) as usize;
        if seg_len < 2 || pos + 2 + seg_len > buf.len() {
            break;
        }

        if marker == 0xDB {
            // DQT segment — may contain multiple tables.
            let seg_data = &buf[pos + 4..pos + 2 + seg_len];
            let mut tbl_pos = 0;
            while tbl_pos < seg_data.len() {
                let prec_id = seg_data[tbl_pos];
                let precision = (prec_id >> 4) & 0x0F; // 0 = 8-bit, 1 = 16-bit
                let dest_id = prec_id & 0x0F;
                let bytes_per_coeff = if precision == 0 { 1_usize } else { 2_usize };
                let table_bytes = 64 * bytes_per_coeff;
                tbl_pos += 1;
                if tbl_pos + table_bytes > seg_data.len() {
                    break;
                }
                let table_data = &seg_data[tbl_pos..tbl_pos + table_bytes];
                let table: Vec<u16> = if precision == 0 {
                    table_data.iter().map(|&b| b as u16).collect()
                } else {
                    table_data
                        .chunks_exact(2)
                        .map(|c| u16::from_be_bytes([c[0], c[1]]))
                        .collect()
                };
                match dest_id {
                    0 => luminance = Some(table),
                    1 => chrominance = Some(table),
                    _ => {}
                }
                tbl_pos += table_bytes;
            }
        }

        pos += 2 + seg_len;
    }

    if luminance.is_none() && chrominance.is_none() {
        return None;
    }

    let estimated_quality = luminance.as_ref().map(|luma| estimate_jpeg_quality(luma));
    let known_source = luminance
        .as_ref()
        .and_then(|luma| match_qtable_signature(luma));

    Some(JpegQuantTables {
        luminance,
        chrominance,
        estimated_quality,
        known_source,
    })
}

/// Estimate the IJG-equivalent quality factor (1–100) from a luminance
/// Q-table by comparing it against the standard Q50 base table.
///
/// The IJG formula: `scale = 50 / Q` for Q < 50, else `scale = 2 - 2*Q/100`.
/// We invert: for each coefficient, `Q_i = base_i / table_i * 50`, then
/// take the median to reduce outlier influence.
fn estimate_jpeg_quality(luma: &[u16]) -> u32 {
    if luma.len() < 64 {
        return 0;
    }
    let mut estimates: Vec<f64> = luma
        .iter()
        .zip(IJG_LUMA_Q50.iter())
        .filter(|(&t, _)| t > 0)
        .map(|(&t, &base)| {
            // scale = base / t  →  Q = 50 / scale  (for scale >= 1, i.e. Q <= 50)
            // scale = base / t  →  Q = 100 - 50 * scale  (for scale < 1, i.e. Q > 50)
            let scale = base as f64 / t as f64;
            if scale >= 1.0 {
                (50.0 / scale).round().clamp(1.0, 100.0)
            } else {
                (100.0 - 50.0 * scale).round().clamp(1.0, 100.0)
            }
        })
        .collect();

    if estimates.is_empty() {
        return 0;
    }
    estimates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = estimates[estimates.len() / 2];
    median as u32
}

/// Try to match a luminance Q-table against the built-in signature database.
/// Uses the first four low-frequency coefficients as a fingerprint.
fn match_qtable_signature(luma: &[u16]) -> Option<String> {
    if luma.len() < 64 {
        return None;
    }
    // 4-coefficient fingerprint: DC + first three AC low-frequency components.
    let sig = [luma[0], luma[1], luma[2], luma[5]];
    // Linear scan — the table is tiny.
    for (pattern, label) in QTABLE_SIGNATURES {
        if sig == *pattern {
            return Some(label.to_string());
        }
    }
    None
}
