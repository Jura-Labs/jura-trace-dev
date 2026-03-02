use std::path::Path;

/// High-level content type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Image,
    Document,
    Video,
    Audio,
    ThreeD,
    Web,
    Unknown,
}

impl ContentType {
    /// String label used in the database and frontend.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Document => "document",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::ThreeD => "3d",
            Self::Web => "web",
            Self::Unknown => "unknown",
        }
    }
}

/// Result of format detection for a single file.
#[derive(Debug)]
pub struct FormatInfo {
    pub content_type: ContentType,
    pub mime_type: String,
}

/// Detect the MIME type and content category of a file.
///
/// Uses the `infer` crate (magic-byte detection) first, then falls back to
/// extension-based lookup so that formats without magic bytes (SVG, Markdown,
/// 3-D meshes, etc.) are still recognised.
pub fn detect(path: &Path) -> FormatInfo {
    // Try magic-byte detection first
    if let Some(kind) = infer::get_from_path(path).ok().flatten() {
        let mime = kind.mime_type().to_string();
        let content_type = classify_mime(&mime);
        return FormatInfo {
            content_type,
            mime_type: mime,
        };
    }

    // Fall back to extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        let (content_type, mime) = classify_extension(&ext_lower);
        return FormatInfo {
            content_type,
            mime_type: mime.to_string(),
        };
    }

    FormatInfo {
        content_type: ContentType::Unknown,
        mime_type: "application/octet-stream".to_string(),
    }
}

/// Classify a MIME string into a content type.
fn classify_mime(mime: &str) -> ContentType {
    if mime.starts_with("image/") {
        ContentType::Image
    } else if mime.starts_with("video/") {
        ContentType::Video
    } else if mime.starts_with("audio/") {
        ContentType::Audio
    } else if mime == "application/pdf"
        || mime == "application/epub+zip"
        || mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        || mime == "application/vnd.oasis.opendocument.text"
    {
        ContentType::Document
    } else if mime == "text/html" || mime == "application/xhtml+xml" {
        ContentType::Web
    } else if mime == "model/gltf+json"
        || mime == "model/gltf-binary"
        || mime == "model/stl"
        || mime == "model/obj"
    {
        ContentType::ThreeD
    } else {
        ContentType::Unknown
    }
}

/// Map file extensions to (ContentType, MIME) for formats that lack magic bytes.
fn classify_extension(ext: &str) -> (ContentType, &'static str) {
    match ext {
        // Images
        "jpg" | "jpeg" => (ContentType::Image, "image/jpeg"),
        "png" => (ContentType::Image, "image/png"),
        "gif" => (ContentType::Image, "image/gif"),
        "webp" => (ContentType::Image, "image/webp"),
        "tiff" | "tif" => (ContentType::Image, "image/tiff"),
        "bmp" => (ContentType::Image, "image/bmp"),
        "svg" => (ContentType::Image, "image/svg+xml"),
        "avif" => (ContentType::Image, "image/avif"),
        "heic" | "heif" => (ContentType::Image, "image/heic"),
        "ico" => (ContentType::Image, "image/x-icon"),

        // Documents
        "pdf" => (ContentType::Document, "application/pdf"),
        "docx" => (ContentType::Document, "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "odt" => (ContentType::Document, "application/vnd.oasis.opendocument.text"),
        "epub" => (ContentType::Document, "application/epub+zip"),
        "txt" => (ContentType::Document, "text/plain"),
        "rtf" => (ContentType::Document, "application/rtf"),
        "md" => (ContentType::Document, "text/markdown"),
        "csv" => (ContentType::Document, "text/csv"),
        "xlsx" => (ContentType::Document, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),

        // Video
        "mp4" | "m4v" => (ContentType::Video, "video/mp4"),
        "mov" => (ContentType::Video, "video/quicktime"),
        "webm" => (ContentType::Video, "video/webm"),
        "avi" => (ContentType::Video, "video/x-msvideo"),
        "mkv" => (ContentType::Video, "video/x-matroska"),

        // Audio
        "wav" => (ContentType::Audio, "audio/wav"),
        "mp3" => (ContentType::Audio, "audio/mpeg"),
        "flac" => (ContentType::Audio, "audio/flac"),
        "ogg" => (ContentType::Audio, "audio/ogg"),
        "aac" => (ContentType::Audio, "audio/aac"),
        "m4a" => (ContentType::Audio, "audio/mp4"),
        "aiff" | "aif" => (ContentType::Audio, "audio/aiff"),
        "opus" => (ContentType::Audio, "audio/opus"),

        // 3D
        "stl" => (ContentType::ThreeD, "model/stl"),
        "obj" => (ContentType::ThreeD, "model/obj"),
        "gltf" => (ContentType::ThreeD, "model/gltf+json"),
        "glb" => (ContentType::ThreeD, "model/gltf-binary"),
        "fbx" => (ContentType::ThreeD, "application/octet-stream"),
        "ply" => (ContentType::ThreeD, "application/octet-stream"),
        "usdz" => (ContentType::ThreeD, "model/vnd.usdz+zip"),
        "3mf" => (ContentType::ThreeD, "application/vnd.ms-package.3dmanufacturing-3dmodel+xml"),
        "dae" => (ContentType::ThreeD, "model/vnd.collada+xml"),

        // Web
        "html" | "htm" => (ContentType::Web, "text/html"),

        _ => (ContentType::Unknown, "application/octet-stream"),
    }
}
