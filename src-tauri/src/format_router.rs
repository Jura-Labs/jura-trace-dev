use std::path::Path;

/// High-level content type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Image,
    Document,
    Video,
    Audio,
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
        "docx" => (
            ContentType::Document,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
        "odt" => (
            ContentType::Document,
            "application/vnd.oasis.opendocument.text",
        ),
        "epub" => (ContentType::Document, "application/epub+zip"),
        "txt" => (ContentType::Document, "text/plain"),
        "rtf" => (ContentType::Document, "application/rtf"),
        "md" => (ContentType::Document, "text/markdown"),
        "csv" => (ContentType::Document, "text/csv"),
        "xlsx" => (
            ContentType::Document,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),

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

        _ => (ContentType::Unknown, "application/octet-stream"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── ContentType::as_str ────────────────────────────────────────

    #[test]
    fn as_str_returns_expected_labels() {
        assert_eq!(ContentType::Image.as_str(), "image");
        assert_eq!(ContentType::Document.as_str(), "document");
        assert_eq!(ContentType::Video.as_str(), "video");
        assert_eq!(ContentType::Audio.as_str(), "audio");
        assert_eq!(ContentType::Unknown.as_str(), "unknown");
    }

    // ── classify_extension ─────────────────────────────────────────

    #[test]
    fn classify_extension_images() {
        for ext in &[
            "jpg", "jpeg", "png", "gif", "webp", "tiff", "tif", "bmp", "svg", "avif", "heic", "ico",
        ] {
            let (ct, _) = classify_extension(ext);
            assert_eq!(ct, ContentType::Image, "extension {ext} should be Image");
        }
    }

    #[test]
    fn classify_extension_documents() {
        for ext in &[
            "pdf", "docx", "odt", "epub", "txt", "rtf", "md", "csv", "xlsx",
        ] {
            let (ct, _) = classify_extension(ext);
            assert_eq!(
                ct,
                ContentType::Document,
                "extension {ext} should be Document"
            );
        }
    }

    #[test]
    fn classify_extension_video() {
        for ext in &["mp4", "m4v", "mov", "webm", "avi", "mkv"] {
            let (ct, _) = classify_extension(ext);
            assert_eq!(ct, ContentType::Video, "extension {ext} should be Video");
        }
    }

    #[test]
    fn classify_extension_audio() {
        for ext in &[
            "wav", "mp3", "flac", "ogg", "aac", "m4a", "aiff", "aif", "opus",
        ] {
            let (ct, _) = classify_extension(ext);
            assert_eq!(ct, ContentType::Audio, "extension {ext} should be Audio");
        }
    }

    #[test]
    fn classify_extension_3d_and_web_fall_to_unknown() {
        // 3D and web extensions are no longer supported — they fall through to Unknown
        for ext in &[
            "stl", "obj", "gltf", "glb", "fbx", "ply", "usdz", "3mf", "dae", "html", "htm",
        ] {
            let (ct, _) = classify_extension(ext);
            assert_eq!(
                ct,
                ContentType::Unknown,
                "extension {ext} should be Unknown"
            );
        }
    }

    #[test]
    fn classify_extension_unknown() {
        let (ct, mime) = classify_extension("zzz");
        assert_eq!(ct, ContentType::Unknown);
        assert_eq!(mime, "application/octet-stream");
    }

    // ── classify_mime ──────────────────────────────────────────────

    #[test]
    fn classify_mime_image() {
        assert_eq!(classify_mime("image/jpeg"), ContentType::Image);
        assert_eq!(classify_mime("image/png"), ContentType::Image);
    }

    #[test]
    fn classify_mime_video() {
        // All standard video MIME types must route to Video
        assert_eq!(classify_mime("video/mp4"), ContentType::Video);
        assert_eq!(classify_mime("video/quicktime"), ContentType::Video);
        assert_eq!(classify_mime("video/webm"), ContentType::Video);
        assert_eq!(classify_mime("video/x-msvideo"), ContentType::Video);
        assert_eq!(classify_mime("video/x-matroska"), ContentType::Video);
    }

    #[test]
    fn classify_mime_audio() {
        // All standard audio MIME types must route to Audio
        assert_eq!(classify_mime("audio/wav"), ContentType::Audio);
        assert_eq!(classify_mime("audio/mpeg"), ContentType::Audio);
        assert_eq!(classify_mime("audio/flac"), ContentType::Audio);
        assert_eq!(classify_mime("audio/ogg"), ContentType::Audio);
        assert_eq!(classify_mime("audio/aac"), ContentType::Audio);
        assert_eq!(classify_mime("audio/mp4"), ContentType::Audio);
    }

    #[test]
    fn classify_mime_document() {
        assert_eq!(classify_mime("application/pdf"), ContentType::Document);
        assert_eq!(classify_mime("application/epub+zip"), ContentType::Document);
    }

    #[test]
    fn classify_mime_web_and_3d_fall_to_unknown() {
        // 3D and web MIME types are no longer supported — they route to Unknown
        assert_eq!(classify_mime("text/html"), ContentType::Unknown);
        assert_eq!(classify_mime("application/xhtml+xml"), ContentType::Unknown);
        assert_eq!(classify_mime("model/gltf+json"), ContentType::Unknown);
        assert_eq!(classify_mime("model/gltf-binary"), ContentType::Unknown);
    }

    #[test]
    fn classify_mime_unknown() {
        assert_eq!(classify_mime("application/zip"), ContentType::Unknown);
    }

    // ── detect() integration ───────────────────────────────────────

    #[test]
    fn detect_falls_back_to_extension() {
        let path = PathBuf::from("/tmp/nonexistent_test_file.svg");
        let info = detect(&path);
        assert_eq!(info.content_type, ContentType::Image);
        assert_eq!(info.mime_type, "image/svg+xml");
    }

    #[test]
    fn detect_unknown_for_no_extension() {
        let path = PathBuf::from("/tmp/nonexistent_test_file");
        let info = detect(&path);
        assert_eq!(info.content_type, ContentType::Unknown);
        assert_eq!(info.mime_type, "application/octet-stream");
    }

    #[test]
    fn detect_unknown_extension() {
        let path = PathBuf::from("/tmp/nonexistent_test_file.xyz123");
        let info = detect(&path);
        assert_eq!(info.content_type, ContentType::Unknown);
    }
}
