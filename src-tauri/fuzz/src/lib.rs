// SPDX-License-Identifier: AGPL-3.0-or-later

//! The app's own parsing modules, compiled as they are in the app.
//!
//! Both files use only external crates, so they build here without the rest
//! of the app. If either starts using `crate::`, this crate stops building,
//! which is the signal to move the harness rather than copy code.

#![allow(dead_code)]

use std::io::Write;
use std::path::PathBuf;

#[path = "../../src/metadata.rs"]
pub mod metadata;

#[path = "../../src/pdf_provenance.rs"]
pub mod pdf_provenance;

/// The parsers take a path, as the app gives them the file the user opened.
/// Each fuzz process writes every input to the same file and reuses it.
pub fn input_file(data: &[u8], ext: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("jura-fuzz-{}.{ext}", std::process::id()));
    let mut f = std::fs::File::create(&path).expect("create fuzz input file");
    f.write_all(data).expect("write fuzz input file");
    path
}
