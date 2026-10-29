#!/bin/bash -eu
# Builds the fuzz targets in src-tauri/fuzz/ for ClusterFuzzLite. The image
# has no src-tauri/Cargo.toml (see the Dockerfile), so cargo-fuzz is run
# from the fuzz crate itself, with --fuzz-dir pointing at it.
cd "$SRC/jura-trace-dev/src-tauri/fuzz"
cargo fuzz build -O --fuzz-dir .
for target in xmp_packet image_metadata pdf_provenance; do
  cp "target/x86_64-unknown-linux-gnu/release/$target" "$OUT/"
done
