use exif::{In, Reader as ExifReader, Tag, Value};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

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
    })
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
