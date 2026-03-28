// `C2pa` and `Internal` are defined for completeness and future use; they are
// not yet wired into all commands. The `log` helper is part of the intended
// public API for call sites that want to log-and-return in one step.
#[allow(dead_code)]
/// Typed, user-safe error enum returned by Tauri IPC commands.
///
/// Raw OS errors (file paths, socket details, rusqlite internals) are logged
/// via [`log::error!`] at the call site and replaced with generic messages
/// before being serialised and sent to the frontend.  This prevents information
/// leakage while still surfacing enough context for the user to act.
///
/// # Design rules
/// - Every variant carries a `String` context for the internal log message.
/// - [`std::fmt::Display`] returns a **generic, user-facing** sentence only.
/// - The raw detail travels only to the process log, never to the frontend.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// A SQLite / rusqlite database operation failed.
    #[error("Database operation failed")]
    Database(String),

    /// A filesystem I/O operation failed (open, read, write, canonicalise).
    #[error("File operation failed")]
    FileSystem(String),

    /// The Python ML sidecar could not be reached or returned an error.
    #[error("Analysis service unavailable")]
    Sidecar(String),

    /// User-supplied input did not pass validation (path traversal, bad URL,
    /// unsupported mode, etc.).  The contained string is already safe to show
    /// to users because it was constructed by our own validation code.
    #[error("{0}")]
    Validation(String),

    /// A C2PA signing or verification operation failed.
    #[error("Content credential operation failed")]
    C2pa(String),

    /// An unexpected internal error with no better category.
    #[error("An internal error occurred")]
    Internal(String),
}

impl AppError {
    /// Log the full detail at ERROR level and return `self`.
    ///
    /// Not yet called by all commands — suppressed until the full error-type
    /// migration is complete.
    #[allow(dead_code)]
    ///
    /// Call this immediately before returning the error so the raw detail is
    /// preserved in the application log while only the generic message crosses
    /// the IPC boundary.
    pub fn log(self) -> Self {
        match &self {
            AppError::Database(detail) => log::error!("Database error: {}", detail),
            AppError::FileSystem(detail) => log::error!("FileSystem error: {}", detail),
            AppError::Sidecar(detail) => log::error!("Sidecar error: {}", detail),
            AppError::Validation(detail) => log::warn!("Validation error: {}", detail),
            AppError::C2pa(detail) => log::error!("C2PA error: {}", detail),
            AppError::Internal(detail) => log::error!("Internal error: {}", detail),
        }
        self
    }
}

/// Serialize `AppError` as a structured JSON object so the frontend can
/// branch on error type without string-sniffing:
///
/// ```json
/// { "code": "Sidecar", "message": "Analysis service unavailable" }
/// ```
///
/// The `code` field is the enum variant name (stable, machine-readable).
/// The `message` field is the `Display` output (user-facing, can change freely).
/// Raw details are never included — they travel only to the process log.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let code = match self {
            AppError::Database(_) => "Database",
            AppError::FileSystem(_) => "FileSystem",
            AppError::Sidecar(_) => "Sidecar",
            AppError::Validation(_) => "Validation",
            AppError::C2pa(_) => "C2pa",
            AppError::Internal(_) => "Internal",
        };
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("code", code)?;
        map.serialize_entry("message", &self.to_string())?;
        map.end()
    }
}

/// Convert a `rusqlite::Error` into [`AppError::Database`].
///
/// The raw error message is stored in the variant for logging; the `Display`
/// impl surfaces only "Database operation failed" to the frontend.
impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

/// Convert a `std::io::Error` into [`AppError::FileSystem`].
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::FileSystem(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_display_is_generic() {
        let err = AppError::Database("no such table: assets".to_string());
        assert_eq!(err.to_string(), "Database operation failed");
        // Ensure the raw detail is not present in the display string
        assert!(!err.to_string().contains("assets"));
    }

    #[test]
    fn filesystem_display_is_generic() {
        let err = AppError::FileSystem(
            "No such file or directory (os error 2): /home/user/secret.jpg".to_string(),
        );
        assert_eq!(err.to_string(), "File operation failed");
        assert!(!err.to_string().contains("secret.jpg"));
    }

    #[test]
    fn sidecar_display_is_generic() {
        let err = AppError::Sidecar("connection refused: 127.0.0.1:8200".to_string());
        assert_eq!(err.to_string(), "Analysis service unavailable");
    }

    #[test]
    fn validation_display_passes_through() {
        // Validation messages are user-authored and safe to surface verbatim.
        let err = AppError::Validation("Invalid file path".to_string());
        assert_eq!(err.to_string(), "Invalid file path");
    }

    #[test]
    fn c2pa_display_is_generic() {
        let err = AppError::C2pa("signing key not found".to_string());
        assert_eq!(err.to_string(), "Content credential operation failed");
    }

    #[test]
    fn internal_display_is_generic() {
        let err = AppError::Internal("mutex poisoned".to_string());
        assert_eq!(err.to_string(), "An internal error occurred");
    }

    #[test]
    fn serialize_returns_structured_object() {
        let err = AppError::Database("raw detail".to_string());
        let json = serde_json::to_string(&err).expect("should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed["code"], "Database");
        assert_eq!(parsed["message"], "Database operation failed");
        // Raw detail must not leak across the IPC boundary
        assert!(!json.contains("raw detail"));
    }

    #[test]
    fn serialize_all_variants_have_correct_code() {
        let cases: Vec<(AppError, &str)> = vec![
            (AppError::Database("d".into()), "Database"),
            (AppError::FileSystem("f".into()), "FileSystem"),
            (AppError::Sidecar("s".into()), "Sidecar"),
            (AppError::Validation("v".into()), "Validation"),
            (AppError::C2pa("c".into()), "C2pa"),
            (AppError::Internal("i".into()), "Internal"),
        ];
        for (err, expected_code) in cases {
            let json = serde_json::to_string(&err).expect("should serialize");
            let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
            assert_eq!(
                parsed["code"], expected_code,
                "wrong code for {}",
                expected_code
            );
            assert!(parsed["message"].is_string(), "message should be a string");
        }
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::FileSystem(_)));
        assert_eq!(app_err.to_string(), "File operation failed");
    }

    #[test]
    fn from_rusqlite_error() {
        // Create a rusqlite error through the public API
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let result = conn.execute("SELECT * FROM nonexistent_table", []);
        let rusqlite_err = result.unwrap_err();
        let app_err: AppError = rusqlite_err.into();
        assert!(matches!(app_err, AppError::Database(_)));
        assert_eq!(app_err.to_string(), "Database operation failed");
    }
}
