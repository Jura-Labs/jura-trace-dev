// SPDX-License-Identifier: AGPL-3.0-or-later

//! The hand-written XMP parser (metadata::parse_xmp_packet). It slices
//! strings at positions found by searching, which is where a bad byte
//! offset would panic.

#![no_main]

use jura_trace_fuzz::metadata;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = metadata::parse_xmp_packet(data);
});
