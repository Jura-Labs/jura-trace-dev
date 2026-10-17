// SPDX-License-Identifier: AGPL-3.0-or-later

//! Where the API address and key come from (design section 5.3):
//!
//! ```text
//! --api-url <url>  >  JURA_API_URL  >  http://127.0.0.1:8300
//! --api-key <key>  >  JURA_API_KEY  >  OS keyring  >  (none)
//! ```
//!
//! `JURA_NO_KEYRING=1` skips the keyring, for CI runners and containers that
//! have no keyring service, and for tests that must not read a developer's
//! stored key.

use crate::exit::{Exit, Failure};

pub const DEFAULT_API_URL: &str = "http://127.0.0.1:8300";
const KEYRING_SERVICE: &str = "org.juralabs.jura";
const KEYRING_ACCOUNT: &str = "api-key";

/// Where a key was found, for `jura auth show-key` and `status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    Flag,
    Env,
    Keyring,
}

impl KeySource {
    pub fn describe(self) -> &'static str {
        match self {
            KeySource::Flag => "the --api-key option",
            KeySource::Env => "the JURA_API_KEY environment variable",
            KeySource::Keyring => "the OS keyring",
        }
    }
}

/// Choose the key by precedence. `keyring` is only called when neither
/// the flag nor the environment supplies one.
pub fn resolve_key(
    flag: Option<&str>,
    env: Option<&str>,
    keyring: impl FnOnce() -> Option<String>,
) -> Option<(String, KeySource)> {
    let non_empty = |v: Option<&str>| {
        v.map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    if let Some(k) = non_empty(flag) {
        return Some((k, KeySource::Flag));
    }
    if let Some(k) = non_empty(env) {
        return Some((k, KeySource::Env));
    }
    keyring().map(|k| (k, KeySource::Keyring))
}

/// Keys the server issues look like `jt_` followed by 64 hex characters.
/// Only the prefix is checked: the server is the authority on the rest.
pub fn check_key_shape(key: &str) -> Result<(), Failure> {
    if key.starts_with("jt_") && key.len() > 3 && !key.chars().any(char::is_whitespace) {
        Ok(())
    } else {
        Err(Failure::new(
            Exit::Auth,
            "that is not a Jura Trace API key: keys start with jt_ and contain no spaces",
        ))
    }
}

/// `jt_1234…cdef`, enough to tell two keys apart without revealing either.
pub fn mask(key: &str) -> String {
    if key.len() <= 11 {
        return "jt_…".to_string();
    }
    format!("{}…{}", &key[..7], &key[key.len() - 4..])
}

pub fn keyring_disabled() -> bool {
    std::env::var("JURA_NO_KEYRING").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn entry() -> Result<keyring::Entry, Failure> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|e| {
        Failure::new(
            Exit::Usage,
            format!("the OS keyring is not available ({e}); use JURA_API_KEY instead"),
        )
    })
}

/// The stored key, or None when there is none or no keyring to ask.
pub fn keyring_get() -> Option<String> {
    if keyring_disabled() {
        return None;
    }
    entry().ok()?.get_password().ok()
}

pub fn keyring_set(key: &str) -> Result<(), Failure> {
    if keyring_disabled() {
        return Err(Failure::new(
            Exit::Usage,
            "JURA_NO_KEYRING is set, so the key was not stored",
        ));
    }
    entry()?.set_password(key).map_err(|e| {
        Failure::new(
            Exit::Usage,
            format!("could not store the key in the OS keyring: {e}"),
        )
    })
}

/// Remove the stored key. True if there was one.
pub fn keyring_clear() -> Result<bool, Failure> {
    if keyring_disabled() {
        return Ok(false);
    }
    match entry()?.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(Failure::new(
            Exit::Usage,
            format!("could not remove the key from the OS keyring: {e}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_is_flag_then_env_then_keyring() {
        let never = || -> Option<String> { panic!("keyring consulted") };
        assert_eq!(
            resolve_key(Some("jt_flag"), Some("jt_env"), never),
            Some(("jt_flag".into(), KeySource::Flag))
        );
        assert_eq!(
            resolve_key(None, Some("jt_env"), never),
            Some(("jt_env".into(), KeySource::Env))
        );
        assert_eq!(
            resolve_key(None, None, || Some("jt_ring".into())),
            Some(("jt_ring".into(), KeySource::Keyring))
        );
        assert_eq!(resolve_key(None, None, || None), None);
    }

    #[test]
    fn an_empty_value_does_not_count() {
        assert_eq!(
            resolve_key(Some(""), Some("  "), || Some("jt_ring".into())),
            Some(("jt_ring".into(), KeySource::Keyring))
        );
    }

    #[test]
    fn key_shape() {
        assert!(check_key_shape("jt_0123abcd").is_ok());
        for bad in ["", "jt_", "0123abcd", "Bearer jt_abc", "jt_ab cd"] {
            let e = check_key_shape(bad).unwrap_err();
            assert_eq!(e.exit, Exit::Auth, "{bad:?}");
        }
    }

    #[test]
    fn masking_never_shows_the_middle() {
        let key = "jt_0123456789abcdef0123456789abcdef";
        let m = mask(key);
        assert_eq!(m, "jt_0123…cdef");
        assert!(!m.contains("89ab"));
        assert_eq!(mask("jt_short"), "jt_…");
    }
}
