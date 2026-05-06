// SPDX-License-Identifier: AGPL-3.0-or-later

//! Build-time code generator — expected detector matrix.
//!
//! Run with:
//!   cargo run --bin gen-detectors -- <repo-root>
//!
//! where <repo-root> is the path to the repository root (the directory that
//! contains `ui/`). Defaults to `..` relative to `src-tauri/` when called
//! from the Makefile / npm `predev`/`prebuild` hooks.
//!
//! Emits `ui/src/lib/generated/expectedDetectors.ts` — a TypeScript module
//! that exports `EXPECTED_DETECTORS_BY_MODE` and the `ContentCategory` type.
//! The file carries a prominent DO-NOT-EDIT header so editors and reviewers
//! know to modify this generator instead of the generated output.
//!
//! ── Contract for detector authors ────────────────────────────────────────
//!
//! The arrays defined in `MODE_MATRIX` below are the single source of truth
//! for "which detectors are expected to run automatically in mode X for
//! content type Y". When you add a new detector to `verify_content_inner` in
//! `src-tauri/src/lib.rs`:
//!
//!   1. Add a `push("your_new_id")` to the `detectors_run_list` block in
//!      `verify_content_inner` (as you would today).
//!   2. Add `"your_new_id"` to the appropriate mode/content-type rows in
//!      `MODE_MATRIX` just below.
//!   3. Re-run this generator (`cargo run --bin gen-detectors -- <repo-root>`)
//!      or let `npm run predev` / `npm run prebuild` do it automatically.
//!   4. Commit both files together.
//!
//! The TypeScript PDF renderer will automatically pick up the new detector
//! in the "Not run in this analysis" rows — NO manual TypeScript edits are
//! needed.
//!
//! On-demand detectors (npr, shadow_consistency, splice_boundary) are NOT
//! included in any automatic row — they are triggered by the user and are
//! excluded by design.
//!
//! `transcription` is listed because it is recorded in `detectors_run` for
//! DB tracking purposes, but it is preprocessing infrastructure rather than
//! a forensic detector. It appears in video/audio rows only.

use std::path::PathBuf;

// ── Detector ID vocabulary ────────────────────────────────────────────────
//
// Must match the string literals pushed in `verify_content_inner` in
// `src-tauri/src/lib.rs` and the IDs stored in the SQLite `detectors_run`
// column (schema v6). IDs are permanent — do not rename without a migration.

/// The content categories used in the expected-detector matrix.
/// Must match the TypeScript `ContentCategory` type in the generated file.
const CONTENT_CATEGORIES: &[&str] = &["image", "video", "audio", "document", "other"];

/// One row in the matrix: (mode, content_category, detectors[]).
/// Every mode × content_category combination must appear exactly once.
/// Ordering within a detectors slice is cosmetic; the PDF renderer uses
/// set membership for the "not run" check.
struct MatrixRow {
    mode: &'static str,
    category: &'static str,
    detectors: &'static [&'static str],
}

/// ── THE SOURCE OF TRUTH ──────────────────────────────────────────────────
///
/// Edit this matrix (and nothing else) when the automatic detector lineup
/// changes. Then re-run the generator to propagate changes to TypeScript.
const MODE_MATRIX: &[MatrixRow] = &[
    // ── quick ─────────────────────────────────────────────────────────────
    // Minimal pass: EXIF + C2PA only. Sidecar is not called.
    MatrixRow {
        mode: "quick",
        category: "image",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "quick",
        category: "video",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "quick",
        category: "audio",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "quick",
        category: "document",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "quick",
        category: "other",
        detectors: &["exif_anomaly", "c2pa"],
    },
    // ── standard ──────────────────────────────────────────────────────────
    // Images: ELA + GBM deepfake + CLIP + watermark (no noise/copy-move/ghost).
    // Video / Audio v1.0: container provenance + metadata only — deepfake +
    // transcription are deferred to v1.0.x (JTV-138/JTV-139).
    MatrixRow {
        mode: "standard",
        category: "image",
        detectors: &[
            "exif_anomaly",
            "c2pa",
            "ela",
            "deepfake",
            "clip",
            "watermark",
        ],
    },
    MatrixRow {
        mode: "standard",
        category: "video",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "standard",
        category: "audio",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "standard",
        category: "document",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "standard",
        category: "other",
        detectors: &["exif_anomaly", "c2pa"],
    },
    // ── deep ──────────────────────────────────────────────────────────────
    // Adds noise, copy-move, JPEG ghost, segmented ELA, colour temperature.
    MatrixRow {
        mode: "deep",
        category: "image",
        detectors: &[
            "exif_anomaly",
            "c2pa",
            "ela",
            "noise",
            "copy_move",
            "deepfake",
            "jpeg_ghost",
            "segmented_ela",
            "colour_temperature",
            "clip",
            "watermark",
        ],
    },
    MatrixRow {
        mode: "deep",
        category: "video",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "deep",
        category: "audio",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "deep",
        category: "document",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "deep",
        category: "other",
        detectors: &["exif_anomaly", "c2pa"],
    },
    // ── archival ──────────────────────────────────────────────────────────
    // Same as deep — archival adds thoroughness on JPEG ghost quality steps,
    // not new detectors. Video deepfake + transcription deferred to v1.0.x.
    MatrixRow {
        mode: "archival",
        category: "image",
        detectors: &[
            "exif_anomaly",
            "c2pa",
            "ela",
            "noise",
            "copy_move",
            "deepfake",
            "jpeg_ghost",
            "segmented_ela",
            "colour_temperature",
            "clip",
            "watermark",
        ],
    },
    MatrixRow {
        mode: "archival",
        category: "video",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "archival",
        category: "audio",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "archival",
        category: "document",
        detectors: &["exif_anomaly", "c2pa"],
    },
    MatrixRow {
        mode: "archival",
        category: "other",
        detectors: &["exif_anomaly", "c2pa"],
    },
];

// ── Validation ────────────────────────────────────────────────────────────

/// Validate that every mode × category combination is present exactly once.
fn validate() {
    let modes = ["quick", "standard", "deep", "archival"];
    for mode in modes {
        for cat in CONTENT_CATEGORIES {
            let count = MODE_MATRIX
                .iter()
                .filter(|r| r.mode == mode && r.category == *cat)
                .count();
            assert!(
                count == 1,
                "MODE_MATRIX: expected exactly one row for ({mode}, {cat}), found {count}",
            );
        }
    }
}

// ── TypeScript emitter ────────────────────────────────────────────────────

fn emit_ts() -> String {
    let mut out = String::with_capacity(4096);

    out.push_str("// DO NOT EDIT — generated by src-tauri/src/bin/gen_detectors.rs\n");
    out.push_str("// Run `cargo run --bin gen-detectors -- <repo-root>` or `npm run predev`\n");
    out.push_str("// to regenerate after changing the detector lineup in Rust.\n");
    out.push_str("//\n");
    out.push_str("// Source of truth: MODE_MATRIX in src-tauri/src/bin/gen_detectors.rs\n");
    out.push_str("// Consumer: ui/src/lib/pdf.ts (PDF \"Not run in this analysis\" rows)\n");
    out.push('\n');

    // ContentCategory type
    let cats: Vec<String> = CONTENT_CATEGORIES
        .iter()
        .map(|c| format!("'{c}'"))
        .collect();
    out.push_str(&format!(
        "export type ContentCategory = {};\n\n",
        cats.join(" | ")
    ));

    // Matrix type alias
    out.push_str("export type ExpectedDetectorMatrix = Record<\n");
    out.push_str("  'quick' | 'standard' | 'deep' | 'archival',\n");
    out.push_str("  Partial<Record<ContentCategory, string[]>>\n");
    out.push_str(">;\n\n");

    // The constant
    out.push_str("export const EXPECTED_DETECTORS_BY_MODE: ExpectedDetectorMatrix = {\n");

    for mode in &["quick", "standard", "deep", "archival"] {
        out.push_str(&format!("  {mode}: {{\n"));
        for cat in CONTENT_CATEGORIES {
            let row = MODE_MATRIX
                .iter()
                .find(|r| r.mode == *mode && r.category == *cat)
                .unwrap_or_else(|| panic!("missing row for ({mode}, {cat})"));
            let ids: Vec<String> = row.detectors.iter().map(|d| format!("'{d}'")).collect();
            out.push_str(&format!("    {cat}: [{}],\n", ids.join(", ")));
        }
        out.push_str("  },\n");
    }

    out.push_str("};\n");
    out
}

fn main() {
    // Validate the matrix first — panics with a clear message on mistakes.
    validate();

    // Determine the output path.
    let repo_root: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        // Default: one level above src-tauri/ when invoked by cargo from the
        // src-tauri directory.
        .unwrap_or_else(|| PathBuf::from(".."));

    let out_path = repo_root
        .join("ui")
        .join("src")
        .join("lib")
        .join("generated")
        .join("expectedDetectors.ts");

    // Ensure the parent directory exists.
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create generated/ directory");
    }

    let ts = emit_ts();
    std::fs::write(&out_path, &ts).expect("Failed to write expectedDetectors.ts");

    println!(
        "gen-detectors: wrote {} ({} bytes)",
        out_path.display(),
        ts.len()
    );
}
