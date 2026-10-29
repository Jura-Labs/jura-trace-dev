// SPDX-License-Identifier: AGPL-3.0-or-later

//! PDF provenance analysis (pdf_provenance::analyse_pdf): lopdf's parser
//! plus our own checks on the header, the trailer and the object tree.

#![no_main]

use jura_trace_fuzz::{input_file, pdf_provenance};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let path = input_file(data, "pdf");
    let _ = pdf_provenance::analyse_pdf(&path);
});
