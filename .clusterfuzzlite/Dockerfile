# ClusterFuzzLite build image for the fuzz targets in src-tauri/fuzz/.
# The base image is pinned by digest; Dependabot's docker entry does not
# cover this directory, so refresh it by hand when ClusterFuzzLite needs a
# newer toolchain:
#   curl -sI -H "Accept: application/vnd.oci.image.index.v1+json" \
#     https://gcr.io/v2/oss-fuzz-base/base-builder-rust/manifests/v1
FROM gcr.io/oss-fuzz-base/base-builder-rust:v1@sha256:3428d0a90b1e7efbe2ba947bd1d0c9a70594986b4b6553804547f1fb24cfc92f

# Only what the fuzz crate compiles: its own files and the two app modules
# it includes by path. The rest of the repo (models, the UI) is not needed.
COPY src-tauri/fuzz $SRC/jura-trace-dev/src-tauri/fuzz
COPY src-tauri/src $SRC/jura-trace-dev/src-tauri/src
COPY .clusterfuzzlite/build.sh $SRC/
WORKDIR $SRC/jura-trace-dev/src-tauri
