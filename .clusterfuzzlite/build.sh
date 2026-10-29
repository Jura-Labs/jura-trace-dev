#!/bin/bash -eu
# Builds the fuzz targets in src-tauri/fuzz/ for ClusterFuzzLite.
cd "$SRC/jura-trace-dev/src-tauri"
cargo fuzz build -O
for target in xmp_packet image_metadata pdf_provenance; do
  cp "fuzz/target/x86_64-unknown-linux-gnu/release/$target" "$OUT/"
done
