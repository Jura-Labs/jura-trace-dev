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
}

/// Extracted metadata from an image file.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    })
}

/// Maximum number of bytes we will read from the head of a file when hunting
/// for an XMP packet. XMP is conventionally placed in the first APP1 segment
/// (JPEG) or the first iTXt chunk (PNG) near the file head, so 2 MB is a
/// very generous cap that still keeps the I/O cost bounded on large files.
const XMP_SCAN_LIMIT: usize = 2 * 1024 * 1024;

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

    Some(XmpMetadata {
        digital_source_type,
        creator_tool,
        credit,
        creator,
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
