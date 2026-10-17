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

    /// Upload one file to `POST /api/v1/protect/sign`. Returns the signed
    /// file's bytes and the filename the server suggests.
    pub fn sign_file(
        &self,
        path: &Path,
        creator: &str,
        license: Option<&str>,
        timestamp: Option<bool>,
    ) -> Result<(Vec<u8>, Option<String>), Failure> {
        check_local_file(path)?;
        let part = reqwest::blocking::multipart::Part::file(path).map_err(|e| {
            Failure::new(Exit::File, format!("{}: cannot read: {e}", path.display()))
        })?;
        let mut form = reqwest::blocking::multipart::Form::new()
            .part("file", part)
            .text("creator_name", creator.to_string());
        if let Some(l) = license {
            form = form.text("license", l.to_string());
        }
        if let Some(t) = timestamp {
            form = form.text("timestamp", if t { "yes" } else { "no" });
        }
        let req = self.authorised(
            self.http
                .post(self.url("/api/v1/protect/sign"))
                .multipart(form),
        )?;
        let resp = req.send().map_err(|e| self.transport_failure(&e))?;
        let status = resp.status();
        let suggested = resp
            .headers()
            .get(reqwest::header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split("filename=").nth(1))
            .map(|f| f.trim_matches(|c| c == '"' || c == ' ').to_string());
        let bytes = resp.bytes().map_err(|e| self.transport_failure(&e))?;
        if !status.is_success() {
            return Err(api_failure(status, &String::from_utf8_lossy(&bytes)));
        }
        if bytes.is_empty() {
            return Err(Failure::new(
                Exit::Server,
                format!("{} answered {status} with an empty file", self.base),
            ));
        }
        Ok((bytes.to_vec(), suggested))
    }

    fn send(&self, req: reqwest::blocking::RequestBuilder) -> Result<Response, Failure> {
        let resp = req.send().map_err(|e| self.transport_failure(&e))?;
        let status = resp.status();
        let raw = resp.text().map_err(|e| self.transport_failure(&e))?;
        if !status.is_success() {
            return Err(api_failure(status, &raw));
        }
        match serde_json::from_str(&raw) {
            Ok(json) => Ok(Response { raw, json }),
            Err(_) => Err(Failure::new(
                Exit::Server,
                format!(
                    "{} answered {status} with a body that is not JSON",
                    self.base
                ),
            )),
        }
    }

    fn transport_failure(&self, e: &reqwest::Error) -> Failure {
        let base = &self.base;
        if e.is_timeout() {
            return Failure::new(
                Exit::Timeout,
                format!("no answer from {base} within the timeout"),
            );
        }
        if e.is_builder() {
            return Failure::new(Exit::Usage, format!("bad API address {base}: {e}"));
        }
        if e.is_connect() || !reachable(base) {
            return Failure::new(
                Exit::Unreachable,
                format!(
                    "could not connect to {base}. Is Jura Trace or jura-trace-api running? \
                     Start one with `jura-trace-api`"
                ),
            );
        }
        // The server is there, but the request broke before its answer
        // could be read. While a file is uploading, the server's middleware
        // can refuse the request (401 for the key, 429 for the rate limit)
        // and close the connection before reading the body; the client then
        // sees a broken pipe and never the response. Found 4 October 2026 on
        // a Linux CI runner, where a rejected key exited 6, not 3. Ask the
        // question again with a request that has no body.
        if let Some(f) = self.probe_key() {
            return f;
        }
        Failure::new(Exit::Server, format!("request to {base} failed: {e}"))
    }

    /// `GET /api/v1/stats` with the key and no body, after an opaque
    /// failure. Returns the failure it reveals, if any.
    fn probe_key(&self) -> Option<Failure> {
        let req = self
            .http
            .get(self.url("/api/v1/stats"))
            .timeout(Duration::from_secs(10));
        let req = match &self.key {
            Some(k) => req.bearer_auth(k),
            None => req,
        };
        match req.send() {
            Ok(resp) if resp.status().is_success() => None,
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body: Option<Value> = resp.json().ok();
                let code = body
                    .as_ref()
                    .and_then(|j| j.get("code"))
                    .and_then(Value::as_str);
                let message = body
                    .as_ref()
                    .and_then(|j| j.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("the server refused the request");
                Some(Failure::new(
                    from_api_error(status, code),
                    format!(
                        "{message} ({}HTTP {status}; the upload was cut off before this answer \
                         could be read)",
                        code.map(|c| format!("{c}, ")).unwrap_or_default()
                    ),
                ))
            }
            Err(_) => None,
        }
    }
}

/// The failure an error response means: its `code` decides, then its HTTP
/// status (design 7.3).
fn api_failure(status: reqwest::StatusCode, raw: &str) -> Failure {
    let json: Option<Value> = serde_json::from_str(raw).ok();
    let code = json
        .as_ref()
        .and_then(|j| j.get("code"))
        .and_then(Value::as_str);
    let message = json
        .as_ref()
        .and_then(|j| j.get("message"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| status.canonical_reason().unwrap_or("error"));
    Failure::new(
        from_api_error(status.as_u16(), code),
        match code {
            Some(code) => format!("{message} ({code}, HTTP {})", status.as_u16()),
            None => format!("{message} (HTTP {})", status.as_u16()),
        },
    )
}

/// Whether anything accepts a TCP connection at the API address. Used when
/// a request fails in a way that hides the cause: with a file to upload,
/// reqwest's blocking client reports a refused connection as a body error
/// ("SendError { kind: Disconnected }"), not as a connect error. Found
/// 4 October 2026 with a 163 KB JPEG, which exited 6 instead of 2.
fn reachable(base: &str) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    let Ok(url) = reqwest::Url::parse(base) else {
        return true;
    };
    let (Some(host), Some(port)) = (url.host_str(), url.port_or_known_default()) else {
        return true;
    };
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let Ok(addrs) = (host, port).to_socket_addrs() else {
        return false;
    };
    addrs
        .into_iter()
        .any(|a| TcpStream::connect_timeout(&a, Duration::from_secs(2)).is_ok())
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
