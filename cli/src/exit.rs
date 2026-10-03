// SPDX-License-Identifier: AGPL-3.0-or-later

//! The exit-code contract (`docs/API_WRAPPER.md`, "CLI exit-code contract";
//! design section 7). Codes are added, never reassigned.
//!
//! Codes 1 to 8 mean no usable verification was produced. 0 and 20 mean one
//! was: a low trust score is not a failure, and exits 0 unless `--fail-on`
//! asked otherwise.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Exit {
    Success = 0,
    /// Bad arguments, an unknown subcommand, conflicting flags, or a
    /// request the server refused as malformed.
    Usage = 1,
    /// Could not connect to the API.
    Unreachable = 2,
    /// No key, a malformed key, or a rejected key.
    Auth = 3,
    /// A local input problem: missing, unreadable, empty, or over the limit.
    File = 4,
    /// The server refused the content as unsupported.
    Format = 5,
    /// The server failed (HTTP 5xx), or answered something unusable.
    Server = 6,
    /// `--timeout` passed, or `--wait-ready` expired with the server up.
    Timeout = 7,
    /// No usable verdict: `--require-complete` and a degraded response, or
    /// `--fail-on` and an `inconclusive` band (decided 3 October 2026).
    Incomplete = 8,
    /// `--fail-on` was given and the verdict met the threshold.
    Verdict = 20,
}

impl Exit {
    pub fn code(self) -> i32 {
        self as i32
    }
}

/// A failure, with the code it exits with and a message for stderr.
#[derive(Debug)]
pub struct Failure {
    pub exit: Exit,
    pub message: String,
}

impl Failure {
    pub fn new(exit: Exit, message: impl Into<String>) -> Self {
        Self {
            exit,
            message: message.into(),
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Map an error response. The `code` field decides, because the server's
/// statuses do not separate caller mistakes from content problems
/// (design 7.3); the HTTP status is the fallback for a code this client
/// does not know, or a body that is not the API's error envelope.
pub fn from_api_error(status: u16, code: Option<&str>) -> Exit {
    match code {
        Some("Unauthorized") => Exit::Auth,
        Some("EmptyFile") | Some("PayloadTooLarge") => Exit::File,
        Some("UnsupportedFormat")
        | Some("FileSystem")
        | Some("C2pa")
        | Some("FingerprintFailed") => Exit::Format,
        Some("MissingField")
        | Some("InvalidParameter")
        | Some("UnsupportedParameter")
        | Some("BadRequest") => Exit::Usage,
        Some("RateLimitExceeded") | Some("ServiceUnavailable") | Some("Internal") => Exit::Server,
        _ => match status {
            401 | 403 => Exit::Auth,
            413 => Exit::File,
            415 | 422 => Exit::Format,
            400..=499 => Exit::Usage,
            _ => Exit::Server,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_published_numbers() {
        let table = [
            (Exit::Success, 0),
            (Exit::Usage, 1),
            (Exit::Unreachable, 2),
            (Exit::Auth, 3),
            (Exit::File, 4),
            (Exit::Format, 5),
            (Exit::Server, 6),
            (Exit::Timeout, 7),
            (Exit::Incomplete, 8),
            (Exit::Verdict, 20),
        ];
        for (exit, n) in table {
            assert_eq!(exit.code(), n, "{exit:?}");
        }
    }

    #[test]
    fn the_code_decides_over_the_status() {
        // An empty file and a malformed request are both HTTP 400.
        assert_eq!(from_api_error(400, Some("EmptyFile")), Exit::File);
        assert_eq!(from_api_error(400, Some("MissingField")), Exit::Usage);
        assert_eq!(from_api_error(422, Some("UnsupportedFormat")), Exit::Format);
        assert_eq!(from_api_error(413, Some("PayloadTooLarge")), Exit::File);
        assert_eq!(from_api_error(401, Some("Unauthorized")), Exit::Auth);
        assert_eq!(from_api_error(429, Some("RateLimitExceeded")), Exit::Server);
        assert_eq!(
            from_api_error(503, Some("ServiceUnavailable")),
            Exit::Server
        );
    }

    #[test]
    fn an_unknown_code_falls_back_to_the_status() {
        assert_eq!(from_api_error(401, None), Exit::Auth);
        assert_eq!(from_api_error(413, Some("SomethingNew")), Exit::File);
        assert_eq!(from_api_error(422, None), Exit::Format);
        assert_eq!(from_api_error(404, None), Exit::Usage);
        assert_eq!(from_api_error(500, None), Exit::Server);
        assert_eq!(from_api_error(502, Some("SomethingNew")), Exit::Server);
    }
}
