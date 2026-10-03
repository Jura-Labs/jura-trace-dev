// SPDX-License-Identifier: AGPL-3.0-or-later

//! The HTTP side: one blocking client, and the translation of every way a
//! request can fail into an exit code.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use crate::exit::{from_api_error, Exit, Failure};

/// The server's upload limit (`RequestBodyLimitLayer` in src-tauri). Checked
/// here so an oversized file fails at once, as a local problem, rather than
/// after uploading 200 MB.
pub const MAX_UPLOAD_BYTES: u64 = 200 * 1024 * 1024;

pub struct Client {
    base: String,
    key: Option<String>,
    http: reqwest::blocking::Client,
}

/// A successful response: the HTTP body, verbatim, and the parsed JSON.
pub struct Response {
    pub raw: String,
    pub json: Value,
}

impl Client {
    /// `timeout` of `None` waits as long as the server takes.
    pub fn new(
        base: &str,
        key: Option<String>,
        timeout: Option<Duration>,
    ) -> Result<Self, Failure> {
        let http = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .user_agent(concat!("jura/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| {
                Failure::new(Exit::Usage, format!("could not start the HTTP client: {e}"))
            })?;
        Ok(Self {
            base: base.trim_end_matches('/').to_string(),
            key,
            http,
        })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    fn authorised(
        &self,
        req: reqwest::blocking::RequestBuilder,
    ) -> Result<reqwest::blocking::RequestBuilder, Failure> {
        match &self.key {
            Some(k) => Ok(req.bearer_auth(k)),
            None => Err(Failure::new(
                Exit::Auth,
                "no API key. Set JURA_API_KEY, or store one with `jura auth set-key`. \
                 To create one: `jura-trace-api keys add --name <label>`",
            )),
        }
    }

    /// `GET` a path that needs no key.
    pub fn get_open(&self, path: &str) -> Result<Response, Failure> {
        self.send(self.http.get(self.url(path)))
    }

    /// `GET` a path with the key.
    pub fn get(&self, path: &str) -> Result<Response, Failure> {
        let req = self.authorised(self.http.get(self.url(path)))?;
        self.send(req)
    }

    /// Upload one file to `POST /api/v1/verify`.
    pub fn verify_file(&self, path: &Path, mode: Option<&str>) -> Result<Response, Failure> {
        check_local_file(path)?;
        let part = reqwest::blocking::multipart::Part::file(path).map_err(|e| {
            Failure::new(Exit::File, format!("{}: cannot read: {e}", path.display()))
        })?;
        let mut form = reqwest::blocking::multipart::Form::new().part("file", part);
        if let Some(m) = mode {
            form = form.text("mode", m.to_string());
        }
        let req = self.authorised(self.http.post(self.url("/api/v1/verify")).multipart(form))?;
        self.send(req)
    }

    /// `POST /api/v1/verify/url`: the server downloads the URL.
    pub fn verify_url(&self, url: &str, mode: Option<&str>) -> Result<Response, Failure> {
        let mut body = serde_json::json!({ "url": url });
        if let Some(m) = mode {
            body["mode"] = Value::String(m.to_string());
        }
        let req = self.authorised(self.http.post(self.url("/api/v1/verify/url")).json(&body))?;
        self.send(req)
    }

    fn send(&self, req: reqwest::blocking::RequestBuilder) -> Result<Response, Failure> {
        let resp = req.send().map_err(|e| transport_failure(&self.base, &e))?;
        let status = resp.status();
        let raw = resp.text().map_err(|e| transport_failure(&self.base, &e))?;
        let json: Option<Value> = serde_json::from_str(&raw).ok();
        if status.is_success() {
            return match json {
                Some(json) => Ok(Response { raw, json }),
                None => Err(Failure::new(
                    Exit::Server,
                    format!(
                        "{} answered {status} with a body that is not JSON",
                        self.base
                    ),
                )),
            };
        }
        let code = json
            .as_ref()
            .and_then(|j| j.get("code"))
            .and_then(Value::as_str);
        let message = json
            .as_ref()
            .and_then(|j| j.get("message"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| status.canonical_reason().unwrap_or("error"));
        let exit = from_api_error(status.as_u16(), code);
        Err(Failure::new(
            exit,
            match code {
                Some(code) => format!("{message} ({code}, HTTP {})", status.as_u16()),
                None => format!("{message} (HTTP {})", status.as_u16()),
            },
        ))
    }
}

fn transport_failure(base: &str, e: &reqwest::Error) -> Failure {
    if e.is_timeout() {
        Failure::new(
            Exit::Timeout,
            format!("no answer from {base} within the timeout"),
        )
    } else if e.is_connect() {
        Failure::new(
            Exit::Unreachable,
            format!(
                "could not connect to {base}. Is Jura Trace or jura-trace-api running? \
                 Start one with `jura-trace-api`"
            ),
        )
    } else if e.is_builder() {
        Failure::new(Exit::Usage, format!("bad API address {base}: {e}"))
    } else {
        Failure::new(Exit::Server, format!("request to {base} failed: {e}"))
    }
}

/// Refuse what the server would refuse anyway, before uploading it.
pub fn check_local_file(path: &Path) -> Result<(), Failure> {
    let meta = std::fs::metadata(path).map_err(|e| {
        let why = match e.kind() {
            std::io::ErrorKind::NotFound => "no such file".to_string(),
            std::io::ErrorKind::PermissionDenied => "permission denied".to_string(),
            _ => e.to_string(),
        };
        Failure::new(Exit::File, format!("{}: {why}", path.display()))
    })?;
    if !meta.is_file() {
        return Err(Failure::new(
            Exit::File,
            format!("{}: not a file", path.display()),
        ));
    }
    if meta.len() == 0 {
        return Err(Failure::new(
            Exit::File,
            format!("{}: the file is empty", path.display()),
        ));
    }
    if meta.len() > MAX_UPLOAD_BYTES {
        return Err(Failure::new(
            Exit::File,
            format!(
                "{}: {} MB is over the server's 200 MB limit",
                path.display(),
                meta.len() / (1024 * 1024)
            ),
        ));
    }
    // Readable, not only present.
    std::fs::File::open(path)
        .map(drop)
        .map_err(|e| Failure::new(Exit::File, format!("{}: cannot read: {e}", path.display())))
}
