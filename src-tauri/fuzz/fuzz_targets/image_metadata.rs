// SPDX-License-Identifier: AGPL-3.0-or-later

//! Everything metadata.rs reads from an image the user opens: EXIF and its
//! thumbnail, XMP, the ICC profile description and the JPEG quantisation
//! tables (the last two are our own parsers), and the dimensions.

#![no_main]

use jura_trace_fuzz::{input_file, metadata};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let path = input_file(data, "jpg");
    let _ = metadata::extract_exif(&path);
    let _ = metadata::extract_xmp(&path);
    let _ = metadata::extract_exif_thumbnail(&path);
    let _ = metadata::extract_icc_profile_description(&path);
    let _ = metadata::extract_jpeg_quant_tables(&path);
    let _ = metadata::get_image_dimensions(&path);
});
