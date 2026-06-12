// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Trust Report PDF generation using jsPDF.
 * Client-side only — no server calls, no canvas dependency.
 */

import { jsPDF } from 'jspdf';
import type { VerificationResult, VerifyMode } from './types';
import { getTrustLevel } from './types';
import { DETECTOR_ID_LABELS } from './detectorLabels';
import {
  C2PA_ACTION_LABELS,
  C2PA_UNKNOWN_ACTION_LABEL,
  C2PA_STATUS_INVALID,
} from './c2pa-labels';
// ── Single source of truth for the expected-detector matrix ──────────────
// Generated from src-tauri/src/bin/gen_detectors.rs (MODE_MATRIX const).
// DO NOT edit this import or the file it points to by hand — run:
//   cargo run --bin gen-detectors -- <repo-root>
// or let `npm run predev` / `npm run prebuild` regenerate it automatically.
import { EXPECTED_DETECTORS_BY_MODE } from './generated/expectedDetectors';
import type { ContentCategory } from './generated/expectedDetectors';

/**
 * Resolve the expected detector IDs for a given mode and content-type string.
 * Returns an empty array when the combination is unrecognised.
 */
function expectedDetectors(mode: string | undefined, contentType: string | undefined): string[] {
  const m = (mode ?? 'standard') as keyof typeof EXPECTED_DETECTORS_BY_MODE;
  const modeMap = EXPECTED_DETECTORS_BY_MODE[m] ?? EXPECTED_DETECTORS_BY_MODE.standard;
  const ct = (contentType ?? 'other') as ContentCategory;
  const key: ContentCategory = ['image', 'video', 'audio', 'document'].includes(ct) ? ct : 'other';
  return modeMap[key] ?? [];
}

export interface ReportMeta {
  fileName: string;
  fileSize: number;
  analysedAt: string;
  analystNote?: string;
  appVersion?: string;
}

/**
 * Analyst declaration fields shown at the top of the PDF.
 * All fields are optional — if none are provided the header block is omitted
 * entirely so Community-tier users who skip the modal see no change.
 */
export interface ReportContext {
  analystName?: string;
  organisation?: string;
  caseReference?: string;
  analysisDate?: string;
}

const PAGE_WIDTH = 210; // A4 mm
const MARGIN = 15;
const CONTENT_WIDTH = PAGE_WIDTH - 2 * MARGIN;
const LINE_HEIGHT = 5;
const SECTION_GAP = 8;

// ── Column positions for signal score tables ─────────────────────
// The four columns are: Detector (widest), Score, Threshold, Status.
// Positions are absolute x values in mm from the left edge of the page.
const COL_DETECTOR = MARGIN;           // left-aligned label (~85 mm available)
const COL_SCORE = MARGIN + 88;         // right of label column
const COL_THRESHOLD = MARGIN + 110;    // next column
const COL_STATUS = MARGIN + 130;       // final column

// ── Academic citations for each detector ─────────────────────────────────
// Cited in the Methodology Disclosure section and the References appendix.
// Keys match the detector description entries in methodologyEntries below.
interface Citation {
  paper: string;
  authors: string;
  year: number;
  /**
   * Fidelity prefix rendered before the inline citation so the report never
   * implies we implement a paper we only follow loosely:
   * 'Adapted from' = our implementation diverges materially from the paper;
   * 'Informed by'  = the paper motivates a simpler heuristic;
   * 'Conforms to'  = a standard/specification, not a paper.
   * Absent = the citation describes the implemented method.
   */
  prefix?: string;
}

const DETECTOR_CITATIONS: Record<string, Citation> = {
  ela:               { paper: 'A Picture\'s Worth: Digital Image Analysis and Forensics', authors: 'Krawetz, N.', year: 2007 },
  noise:             { paper: 'Using Noise Inconsistencies for Blind Image Forensics', authors: 'Mahdian, B. & Saic, S.', year: 2009 },
  copyMove:          { paper: 'Distinctive Image Features from Scale-Invariant Keypoints', authors: 'Lowe, D.G.', year: 2004 },
  deepfake:          { paper: 'Greedy Function Approximation: A Gradient Boosting Machine', authors: 'Friedman, J.H.', year: 2001 },
  clipDetect:        { paper: 'Towards Universal Fake Image Detectors that Generalise Across Generative Models', authors: 'Ojha, U. et al.', year: 2023 },
  univfd:            { paper: 'Towards Universal Fake Image Detectors that Generalise Across Generative Models', authors: 'Ojha, U. et al.', year: 2023 },
  jpegGhost:         { paper: 'Exposing Digital Forgeries from JPEG Ghosts', authors: 'Farid, H.', year: 2009 },
  segmentedEla:      { paper: 'A Picture\'s Worth: Digital Image Analysis and Forensics', authors: 'Krawetz, N.', year: 2007, prefix: 'Regional application of' },
  shadowConsistency: { paper: 'Exposing Photo Manipulation with Inconsistent Shadows', authors: "Kee, E., O'Brien, J.F. & Farid, H.", year: 2013, prefix: 'Adapted from' },
  colourTemperature: { paper: 'Exposing Digital Image Forgeries by Illuminant Color Classification', authors: 'de Carvalho, T.J. et al.', year: 2013, prefix: 'Informed by' },
  spliceBoundary:    { paper: 'Multi-signal splice boundary heuristic ensemble (no external citation)', authors: 'Jura Trace', year: 2026 },
  npr:               { paper: 'Rethinking the Up-Sampling Operations in CNN-based Generative Network for Generalizable Deepfake Detection', authors: 'Tan, C. et al.', year: 2024, prefix: 'Adapted from' },
  c2pa:              { paper: 'C2PA Technical Specification, version 2.2', authors: 'Coalition for Content Provenance and Authenticity', year: 2024, prefix: 'Conforms to' },
  watermark:         { paper: 'Robust Image Watermarking Using DWT-DCT-SVD', authors: 'Navas, K.A. et al.', year: 2008 },
  exifAnomaly:       { paper: 'Exchangeable image file format for digital still cameras: Exif Version 3.0 (CIPA DC-008-2023) and IPTC Photo Metadata Standard', authors: 'Camera & Imaging Products Association, JEITA & IPTC', year: 2023, prefix: 'Conforms to' },
};

// Known thresholds for primary detectors (mirrors Python sidecar defaults).
// Detectors without a single fixed published threshold use null — shown as "—".
const DETECTOR_THRESHOLDS: Record<string, number | null> = {
  ela: 0.30,
  noise: 0.25,
  copyMove: 0.10,
  deepfake: 0.50,
  npr: null,
  jpegGhost: null,
};

/** Report format: 'standard' for the default report, 'berkeley' for Berkeley Protocol legal evidence format. */
export type ReportFormat = 'standard' | 'berkeley';

/** Transcode a heatmap image (identified by its on-disk path) to a JPEG data
 *  URL for PDF embedding, downscaled to `maxWidth` px.
 *
 *  In Tauri the path is converted to an asset:// URL via `convertFileSrc()`
 *  so the webview can load it. An empty or falsy path is rejected immediately.
 *  White background is painted first because JPEG has no alpha channel.
 */
async function transcodePngToJpeg(
  pathOrUrl: string,
  maxWidth = 800,
  quality = 0.75,
): Promise<string> {
  if (!pathOrUrl) throw new Error('empty heatmap path');
  let src: string;
  if (pathOrUrl.startsWith('data:') || pathOrUrl.startsWith('http')) {
    src = pathOrUrl;
  } else {
    try {
      const { convertFileSrc } = await import('@tauri-apps/api/core');
      src = convertFileSrc(pathOrUrl);
    } catch {
      src = pathOrUrl;
    }
  }
  const img = new Image();
  img.src = src;
  await new Promise<void>((resolve, reject) => {
    img.onload = () => resolve();
    img.onerror = () => reject(new Error('heatmap decode failed'));
  });
  const scale = Math.min(1, maxWidth / img.naturalWidth);
  const w = Math.max(1, Math.round(img.naturalWidth * scale));
  const h = Math.max(1, Math.round(img.naturalHeight * scale));
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const cctx = canvas.getContext('2d');
  if (!cctx) throw new Error('canvas 2d context unavailable');
  cctx.fillStyle = '#ffffff';
  cctx.fillRect(0, 0, w, h);
  cctx.drawImage(img, 0, 0, w, h);
  return canvas.toDataURL('image/jpeg', quality);
}

/** Generate a trust report PDF and return as a Blob. */
export async function generateTrustReport(result: VerificationResult, meta: ReportMeta, ctx?: ReportContext, reportFormat: ReportFormat = 'standard'): Promise<Blob> {
  const doc = new jsPDF({ unit: 'mm', format: 'a4', compress: true });
  const version = meta.appVersion ?? '0.9.0';
  let y = MARGIN;

  function addFooter() {
    const pageCount = doc.getNumberOfPages();
    for (let i = 1; i <= pageCount; i++) {
      doc.setPage(i);
      doc.setFontSize(7);
      doc.setTextColor(120);
      doc.text(
        `Generated by Jura Trace v${version} — Jura Labs CIC — Local processing only. No data transmitted.`,
        MARGIN, 287
      );
      doc.text(`Page ${i} of ${pageCount}`, PAGE_WIDTH - MARGIN, 287, { align: 'right' });
    }
  }

  function checkPage(needed: number) {
    if (y + needed > 270) {
      doc.addPage();
      y = MARGIN;
    }
  }

  function heading(text: string) {
    checkPage(12);
    doc.setFontSize(11);
    doc.setTextColor(40);
    doc.setFont('helvetica', 'bold');
    doc.text(text, MARGIN, y);
    y += 6;
    doc.setDrawColor(180);
    doc.line(MARGIN, y, PAGE_WIDTH - MARGIN, y);
    y += 4;
    doc.setFont('helvetica', 'normal');
  }

  function label(text: string) {
    doc.setFontSize(8);
    doc.setTextColor(100);
    doc.text(text, MARGIN, y);
  }

  function value(text: string, xOffset = 45) {
    doc.setFontSize(9);
    doc.setTextColor(40);
    doc.text(text, MARGIN + xOffset, y);
    y += LINE_HEIGHT;
  }

  function row(labelText: string, valueText: string) {
    checkPage(LINE_HEIGHT + 1);
    label(labelText);
    value(valueText);
  }

  function paragraph(text: string, fontSize = 8) {
    checkPage(10);
    doc.setFontSize(fontSize);
    doc.setTextColor(60);
    const lines = doc.splitTextToSize(text, CONTENT_WIDTH);
    doc.text(lines, MARGIN, y);
    y += lines.length * (fontSize * 0.4) + 2;
  }

  /** Render the four-column header for a signal score table. */
  function signalTableHeader() {
    checkPage(10);
    doc.setFontSize(7);
    doc.setFont('helvetica', 'bold');
    doc.setTextColor(80);
    doc.text('Detector', COL_DETECTOR, y);
    doc.text('Score', COL_SCORE, y);
    doc.text('Threshold', COL_THRESHOLD, y);
    doc.text('Status', COL_STATUS, y);
    y += 3;
    doc.setDrawColor(160);
    doc.setLineWidth(0.2);
    doc.line(MARGIN, y, PAGE_WIDTH - MARGIN, y);
    y += 3;
    doc.setFont('helvetica', 'normal');
  }

  /**
   * Render a single row in a signal score table.
   * @param detectorLabel  Human-readable detector name
   * @param score          Normalised score 0.0–1.0, or null/undefined if not run
   * @param threshold      Fixed threshold 0.0–1.0, or null if no published threshold
   * @param suspicious     Result-level suspicious flag; used when threshold is null
   * @param notRunOverride When true, the row spans all data columns with the
   *                       italic grey label "Not run in this analysis", replacing
   *                       score/threshold/status cells. Used when a detector is
   *                       expected for the current mode/content-type but is absent
   *                       from result.detectorsRun.
   */
  function signalTableRow(
    detectorLabel: string,
    score: number | null | undefined,
    threshold: number | null,
    suspicious: boolean | null | undefined,
    notRunOverride = false
  ) {
    checkPage(LINE_HEIGHT + 1);

    doc.setFontSize(8);
    doc.setTextColor(50);
    doc.text(detectorLabel, COL_DETECTOR, y);

    if (notRunOverride) {
      // Italic grey spanning the three data columns
      doc.setFontSize(7);
      doc.setFont('helvetica', 'italic');
      doc.setTextColor(140);
      doc.text('Not run in this analysis', COL_SCORE, y);
      doc.setFont('helvetica', 'normal');
      doc.setTextColor(40);
    } else {
      const notRun = score == null;
      const scoreText = notRun ? '\u2014' : score.toFixed(2);
      const thresholdText = threshold != null ? threshold.toFixed(2) : '\u2014';

      let statusText: string;
      let statusR: number;
      let statusG: number;
      let statusB: number;
      if (notRun) {
        statusText = '\u2014';
        statusR = 140; statusG = 140; statusB = 140;
      } else if (threshold != null) {
        if (score! >= threshold) {
          statusText = 'Flagged';
          statusR = 180; statusG = 60; statusB = 60;
        } else {
          statusText = 'Clean';
          statusR = 40; statusG = 120; statusB = 60;
        }
      } else {
        // No fixed threshold — use the result's own suspicious flag
        if (suspicious === true) {
          statusText = 'Flagged';
          statusR = 180; statusG = 60; statusB = 60;
        } else if (suspicious === false) {
          statusText = 'Clean';
          statusR = 40; statusG = 120; statusB = 60;
        } else {
          statusText = '\u2014';
          statusR = 140; statusG = 140; statusB = 140;
        }
      }

      doc.setTextColor(40);
      doc.text(scoreText, COL_SCORE, y);
      doc.text(thresholdText, COL_THRESHOLD, y);

      doc.setTextColor(statusR, statusG, statusB);
      doc.text(statusText, COL_STATUS, y);
      doc.setTextColor(40); // reset
    }

    y += LINE_HEIGHT;

    // Light hairline separator between rows
    doc.setDrawColor(220);
    doc.setLineWidth(0.1);
    doc.line(MARGIN, y - 1, PAGE_WIDTH - MARGIN, y - 1);
  }

  // ── Header ──────────────────────────────────────────────────
  if (reportFormat === 'berkeley') {
    // Berkeley Protocol evidence documentation header
    doc.setDrawColor(40);
    doc.setLineWidth(0.6);
    doc.line(MARGIN, y, PAGE_WIDTH - MARGIN, y);
    y += 6;

    doc.setFontSize(12);
    doc.setTextColor(30);
    doc.setFont('helvetica', 'bold');
    doc.text('DIGITAL EVIDENCE AUTHENTICATION REPORT', MARGIN, y);
    y += 6;

    doc.setFontSize(8);
    doc.setFont('helvetica', 'normal');
    doc.setTextColor(60);
    doc.text('Berkeley Protocol on Digital Open Source Investigations (2020)', MARGIN, y);
    y += 5;

    doc.setDrawColor(40);
    doc.setLineWidth(0.6);
    doc.line(MARGIN, y, PAGE_WIDTH - MARGIN, y);
    y += 5;

    // Case metadata block
    doc.setFontSize(8);
    if (ctx?.caseReference?.trim()) {
      doc.setTextColor(100);
      doc.text('Case Reference:', MARGIN, y);
      doc.setTextColor(40);
      doc.text(ctx.caseReference.trim(), MARGIN + 30, y);
      y += LINE_HEIGHT;
    }
    if (ctx?.analysisDate?.trim()) {
      doc.setTextColor(100);
      doc.text('Date of Analysis:', MARGIN, y);
      doc.setTextColor(40);
      doc.text(ctx.analysisDate.trim(), MARGIN + 30, y);
      y += LINE_HEIGHT;
    }
    if (ctx?.analystName?.trim()) {
      doc.setTextColor(100);
      doc.text('Analyst:', MARGIN, y);
      doc.setTextColor(40);
      doc.text(ctx.analystName.trim(), MARGIN + 30, y);
      y += LINE_HEIGHT;
    }
    if (ctx?.organisation?.trim()) {
      doc.setTextColor(100);
      doc.text('Organisation:', MARGIN, y);
      doc.setTextColor(40);
      doc.text(ctx.organisation.trim(), MARGIN + 30, y);
      y += LINE_HEIGHT;
    }
    doc.setTextColor(100);
    doc.text('Report Generated:', MARGIN, y);
    doc.setTextColor(40);
    doc.text(new Date().toISOString(), MARGIN + 30, y);
    y += LINE_HEIGHT;

    doc.setDrawColor(40);
    doc.setLineWidth(0.3);
    doc.line(MARGIN, y + 1, PAGE_WIDTH - MARGIN, y + 1);
    y += SECTION_GAP;
  } else {
    doc.setFontSize(16);
    doc.setTextColor(30);
    doc.setFont('helvetica', 'bold');
    doc.text('Jura Trace', MARGIN, y);
    y += 6;
    doc.setFontSize(10);
    doc.setFont('helvetica', 'normal');
    doc.setTextColor(80);
    doc.text('Content Verification Report', MARGIN, y);
    y += 4;
    doc.setFontSize(8);
    doc.text(`Jura Labs CIC — ${new Date(meta.analysedAt).toLocaleString('en-GB')}`, MARGIN, y);
    y += SECTION_GAP;
  }

  // ── Analyst Declaration (only when at least one field is populated) ──
  const hasDeclaration = ctx && (
    (ctx.analystName?.trim()) ||
    (ctx.organisation?.trim()) ||
    (ctx.caseReference?.trim()) ||
    (ctx.analysisDate?.trim())
  );

  if (hasDeclaration && ctx) {
    // Top rule
    doc.setDrawColor(60);
    doc.setLineWidth(0.4);
    doc.line(MARGIN, y, PAGE_WIDTH - MARGIN, y);
    y += 5;

    doc.setFontSize(9);
    doc.setTextColor(40);
    doc.setFont('helvetica', 'bold');
    doc.text('FORENSIC ANALYSIS REPORT', MARGIN, y);
    y += 5;

    // Divider
    doc.setDrawColor(60);
    doc.setLineWidth(0.4);
    doc.line(MARGIN, y, PAGE_WIDTH - MARGIN, y);
    y += 5;

    doc.setFont('helvetica', 'normal');

    // Analyst + Date on the same row when both present
    if (ctx.analystName?.trim() && ctx.analysisDate?.trim()) {
      doc.setFontSize(8);
      doc.setTextColor(100);
      doc.text('Analyst:', MARGIN, y);
      doc.setTextColor(40);
      doc.text(ctx.analystName.trim(), MARGIN + 20, y);
      doc.setTextColor(100);
      doc.text('Date:', PAGE_WIDTH / 2, y);
      doc.setTextColor(40);
      doc.text(ctx.analysisDate.trim(), PAGE_WIDTH / 2 + 12, y);
      y += LINE_HEIGHT;
    } else {
      if (ctx.analystName?.trim()) {
        doc.setFontSize(8);
        doc.setTextColor(100);
        doc.text('Analyst:', MARGIN, y);
        doc.setTextColor(40);
        doc.text(ctx.analystName.trim(), MARGIN + 20, y);
        y += LINE_HEIGHT;
      }
      if (ctx.analysisDate?.trim()) {
        doc.setFontSize(8);
        doc.setTextColor(100);
        doc.text('Date:', MARGIN, y);
        doc.setTextColor(40);
        doc.text(ctx.analysisDate.trim(), MARGIN + 20, y);
        y += LINE_HEIGHT;
      }
    }

    if (ctx.organisation?.trim()) {
      doc.setFontSize(8);
      doc.setTextColor(100);
      doc.text('Organisation:', MARGIN, y);
      doc.setTextColor(40);
      doc.text(ctx.organisation.trim(), MARGIN + 27, y);
      y += LINE_HEIGHT;
    }

    if (ctx.caseReference?.trim()) {
      doc.setFontSize(8);
      doc.setTextColor(100);
      doc.text('Case Reference:', MARGIN, y);
      doc.setTextColor(40);
      doc.text(ctx.caseReference.trim(), MARGIN + 31, y);
      y += LINE_HEIGHT;
    }

    // Bottom rule
    y += 2;
    doc.setDrawColor(60);
    doc.setLineWidth(0.4);
    doc.line(MARGIN, y, PAGE_WIDTH - MARGIN, y);
    y += SECTION_GAP;
  }

  // ── Berkeley: Capture Environment ───────────────────────────
  if (reportFormat === 'berkeley') {
    heading('Capture Environment');

    // Mode label normalisation: 'archival' was retired 2026-04-22 and
    // aliased to 'deep' in the Rust backend. Render legacy archival
    // result records as "Deep" so the PDF doesn't claim a mode the
    // current build no longer offers — the analysis was the same.
    const berkeleyRawMode = result.mode ?? 'standard';
    const berkeleyAnalysisMode =
      berkeleyRawMode === 'archival' ? 'Deep' :
      berkeleyRawMode === 'deep' ? 'Deep' :
      'Standard';

    const berkeleyDetectors: string[] = [];
    if (result.elaResult) berkeleyDetectors.push('ELA');
    if (result.noiseResult) berkeleyDetectors.push('Noise Analysis');
    if (result.copyMoveResult) berkeleyDetectors.push('Copy-Move Detection');
    if (result.deepfakeResult) berkeleyDetectors.push('AI Generation Detection');
    if (result.nprResult) berkeleyDetectors.push('NPR');
    if (result.jpegGhostResult) berkeleyDetectors.push('JPEG Ghost');
    if (result.segmentedElaResult) berkeleyDetectors.push('Segmented ELA');
    if (result.shadowConsistencyResult) berkeleyDetectors.push('Shadow Consistency');
    if (result.colourTemperatureResult) berkeleyDetectors.push('Colour Temperature');
    if (result.spliceBoundaryResult) berkeleyDetectors.push('Splice Boundary');
    if (result.clipResult) berkeleyDetectors.push('CLIP Detection');
    if (result.exifAnalysis) berkeleyDetectors.push('EXIF Anomaly Analysis');
    if (result.c2paManifest !== undefined) berkeleyDetectors.push('C2PA Credential Verification');

    row('Platform', `Jura Trace v${version}`);
    row('Analysis Mode', berkeleyAnalysisMode);
    row('Detectors Executed', berkeleyDetectors.length > 0 ? String(berkeleyDetectors.length) : '0');
    checkPage(10);
    doc.setFontSize(7);
    doc.setTextColor(60);
    const detList = doc.splitTextToSize(berkeleyDetectors.join(', ') || 'None', CONTENT_WIDTH - 2);
    doc.text(detList, MARGIN + 2, y);
    y += detList.length * 3 + 2;
    row('Offline Analysis', 'Confirmed — all processing performed locally on-device');
    y += SECTION_GAP;
  }

  // ── Summary ─────────────────────────────────────────────────
  heading('Summary');
  const trustPercent = Math.round(result.overallTrust * 100);
  const trustLevel = getTrustLevel(result.overallTrust);
  row('File', meta.fileName);
  row('File Size', formatBytes(meta.fileSize));
  row('Trust Score', `${trustPercent}% (${trustLevel === 'high' ? 'High Trust' : trustLevel === 'medium' ? 'Moderate Trust' : 'Low Trust'})`);
  row('Source Type', result.sourceType);
  row('Content Type', result.contentType);
  if (result.aiGenerator) {
    row('AI Generator', result.aiGenerator);
  }

  y += SECTION_GAP;

  // ── Partial Analysis Notice (Niamh fix, 2026-05-12) ─────────
  // When fewer detectors ran than expected for this mode + content-type,
  // the report is provisional. The notice appears at the front of the
  // report — right after the Summary, before the analyst note — so a
  // solicitor / audit reviewer cannot accept the analysis without first
  // confirming the scope. Hidden when analysis ran to full expected scope.
  //
  // Backed by `result.detectorsRun` (S28-FU1, JTV-181) against
  // `expectedDetectors(mode, contentType)` from the codegen-stable
  // EXPECTED_DETECTORS_BY_MODE matrix.
  const expectedIdsForNotice = expectedDetectors(result.mode, result.contentType);
  const expectedCount = expectedIdsForNotice.length;
  const actualCount = result.detectorsRun?.length ?? 0;
  const isPartialAnalysis = expectedCount > 0 && actualCount < expectedCount;
  const noticeModeLabel =
    (result.mode ?? 'standard') === 'archival' || (result.mode ?? 'standard') === 'deep'
      ? 'Deep' : 'Standard';

  if (isPartialAnalysis) {
    checkPage(30);
    const bannerY = y;
    const bannerH = 26;
    doc.setFillColor(252, 232, 232);    // light cinnabar tint
    doc.setDrawColor(160, 60, 50);      // cinnabar border
    doc.setLineWidth(0.6);
    doc.rect(MARGIN, bannerY, CONTENT_WIDTH, bannerH, 'FD');

    doc.setFont('helvetica', 'bold');
    doc.setFontSize(10);
    doc.setTextColor(140, 35, 30);
    doc.text('PARTIAL ANALYSIS NOTICE', MARGIN + 3, bannerY + 6);

    doc.setFont('helvetica', 'normal');
    doc.setFontSize(8);
    doc.setTextColor(70, 25, 25);
    const partialMsg =
      `This analysis ran ${actualCount} of ${expectedCount} expected detectors for ` +
      `${result.contentType} content in ${noticeModeLabel} mode. ` +
      'The report is provisional — detectors marked "Not run in this analysis" did ' +
      'not contribute. Re-run with full Analysis Engine availability before ' +
      'treating any verdict as definitive evidence.';
    const msgLines = doc.splitTextToSize(partialMsg, CONTENT_WIDTH - 6);
    doc.text(msgLines, MARGIN + 3, bannerY + 11);

    doc.setTextColor(0);
    doc.setFont('helvetica', 'normal');
    y = bannerY + bannerH + SECTION_GAP;
  }

  // ── Analyst Notes (dedicated section — only when note is non-empty) ──
  if (meta.analystNote?.trim()) {
    heading('Analyst Notes');
    // Split on explicit newlines first, then word-wrap each paragraph
    const noteParas = meta.analystNote.trim().split(/\r?\n/);
    for (const para of noteParas) {
      const trimmed = para.trim();
      if (trimmed.length === 0) {
        // Blank line between paragraphs — add a small gap
        y += 3;
        continue;
      }
      checkPage(10);
      doc.setFontSize(8);
      doc.setTextColor(50);
      const wrappedLines = doc.splitTextToSize(trimmed, CONTENT_WIDTH);
      doc.text(wrappedLines, MARGIN, y);
      y += wrappedLines.length * (8 * 0.4) + 2;
    }
    y += SECTION_GAP;
  }

  // ── Berkeley: Evidence Integrity ────────────────────────────
  if (reportFormat === 'berkeley') {
    heading('Evidence Integrity');
    if (result.inputSha256) {
      row('Input SHA-256', result.inputSha256);
    } else {
      row('Input SHA-256', 'Not computed');
    }
    row('File Name', meta.fileName);
    row('File Size', formatBytes(meta.fileSize));
    row('MIME Type', result.contentType || 'Unknown');
    y += 2;
    paragraph(
      'Chain of custody: This file was analysed locally on the analyst\u2019s device using Jura Trace. ' +
      'No copy of the file was transmitted to any external server during analysis. ' +
      'The SHA-256 hash above can be used to verify that any subsequent copy matches the file as analysed.',
      7
    );
    y += SECTION_GAP;
  }

  // ── C2PA Credentials ────────────────────────────────────────
  // Renders a label/value pair where the value wraps onto subsequent lines
  // if it exceeds the column width. Used for fields that can be arbitrarily
  // long (signer CN, claim generator, Table 4 status strings).
  function wrappedRow(labelText: string, valueText: string) {
    checkPage(LINE_HEIGHT + 1);
    doc.setFontSize(8);
    doc.setTextColor(100);
    doc.text(labelText, MARGIN, y);
    doc.setFontSize(9);
    doc.setTextColor(40);
    const valLines = doc.splitTextToSize(valueText, CONTENT_WIDTH - 45);
    doc.text(valLines, MARGIN + 45, y);
    y += Math.max(LINE_HEIGHT, valLines.length * 4 + 1);
  }

  heading('C2PA Provenance');
  if (result.c2paManifest) {
    const m = result.c2paManifest;
    wrappedRow(
      'Status',
      m.isValid
          ? 'Valid'
          : C2PA_STATUS_INVALID
    );
    // Verification mode sits next to Status — related concept, not buried after assertions.
    if (m.verificationMode) {
      wrappedRow(
        'Verification mode',
        m.verificationMode === 'enhanced'
          ? 'Enhanced (OCSP/CRL + remote manifest fetch)'
          : 'Standard (offline, local trust anchors only)'
      );
    }
    // C2PA UX Rec v1.4 Table 5 consumer-friendly labels.
    if (m.appOrDevice) wrappedRow('App or device used', m.appOrDevice);
    if (m.signedBy) {
      const via = m.signedByIssuer ? ` (via ${m.signedByIssuer})` : '';
      wrappedRow('Issued by', m.signedBy + via);
    } else if (m.claimGenerator && !m.appOrDevice) {
      // Fall back to claim generator only when no richer label is available.
      wrappedRow('App or device used', m.claimGenerator);
    }
    if (m.signedAt) wrappedRow('Date', new Date(m.signedAt).toLocaleString('en-GB'));
    if (m.format) wrappedRow('Format', m.format);
    if (m.title) wrappedRow('Title', m.title);

    // Validation summary — human-readable translation of the raw c2pa-rs
    // check codes, mirroring the verify page's L3 panel (v2 +page.svelte
    // lines 2563–2605). The old per-code dump was L4 content in an L3 section
    // and overflowed the page when codes were long.
    if (m.validationChecks && m.validationChecks.length > 0) {
      const checks = m.validationChecks;
      const has = (code: string, outcome: 'pass' | 'fail' | 'info') =>
        checks.some(c => c.code === code && c.outcome === outcome);
      const hasMatch = (frag: string, outcome: 'pass' | 'fail' | 'info') =>
        checks.some(c => c.code.includes(frag) && c.outcome === outcome);

      const sigValid = has('claimSignature.validated', 'pass');
      const dataValid = has('assertion.dataHash.match', 'pass');
      const tsValid =
        has('timeStamp.validated', 'pass') || has('timeStamp.trusted', 'pass');
      const hashFail = hasMatch('dataHash.mismatch', 'fail');

      y += 3;
      checkPage(12);
      doc.setFontSize(8);
      doc.setTextColor(80);
      doc.text('Validation summary', MARGIN, y);
      y += LINE_HEIGHT;

      // ASCII marker instead of ✓/✗/⚠ — WinAnsi Helvetica (jsPDF default)
      // does not map U+2713/U+2717/U+26A0 and renders them as stray glyphs.
      // Colour still conveys the outcome.
      function summaryLine(
        marker: string,
        r: number, g: number, b: number,
        text: string,
      ) {
        checkPage(LINE_HEIGHT);
        doc.setFontSize(8);
        doc.setTextColor(r, g, b);
        doc.text(marker, MARGIN + 2, y);
        doc.setTextColor(50);
        const lines = doc.splitTextToSize(text, CONTENT_WIDTH - 14);
        doc.text(lines, MARGIN + 12, y);
        y += Math.max(LINE_HEIGHT, lines.length * 4);
      }

      // Green / red / amber / neutral colour triples (pass / fail / warn / info).
      const OK: [number, number, number] = [91, 138, 95];
      const FAIL: [number, number, number] = [205, 92, 92];
      const WARN: [number, number, number] = [180, 140, 50];
      const INFO: [number, number, number] = [140, 140, 140];

      summaryLine(
        sigValid ? '[OK]' : '[FAIL]',
        ...(sigValid ? OK : FAIL),
        sigValid ? 'Signature valid' : C2PA_STATUS_INVALID,
      );
      summaryLine(
        dataValid ? '[OK]' : hashFail ? '[FAIL]' : '[-]',
        ...(dataValid ? OK : hashFail ? FAIL : INFO),
        dataValid
          ? 'Data integrity confirmed - file has not been modified'
          : hashFail
            ? 'Data integrity failed - file has been modified since signing'
            : 'Data hash not checked',
      );
      if (tsValid) {
        summaryLine('[OK]', ...OK, 'Timestamp verified');
      }
    }

    // Assertions block — consumer-facing summary only. Structural hash
    // assertions (c2pa.hash.data*, c2pa.hash.multi-asset) are skipped here
    // because they dump hundreds of lines of base64/exclusion JSON that
    // belong in an L4 technical appendix, not the main report body.
    // The claim generator, actions, and creative-work assertions carry the
    // meaningful provenance information.
    if (m.assertions.length > 0) {
      const contentAssertions = m.assertions.filter(a => {
        const label = a.label;
        if (label.startsWith('c2pa.hash.')) return false;
        if (label === 'c2pa.claim.v2' || label === 'c2pa.claim') return false;
        return true;
      });
      if (contentAssertions.length > 0) {
        y += 3;
        checkPage(10);
        doc.setFontSize(8);
        doc.setTextColor(80);
        doc.text(`Assertions (${contentAssertions.length}):`, MARGIN, y);
        y += LINE_HEIGHT;
        for (const a of contentAssertions) {
          checkPage(LINE_HEIGHT * 2);
          doc.setFontSize(7);
          doc.setTextColor(70);
          doc.text(a.label, MARGIN + 2, y);
          y += 3.5;
          const valLines = doc.splitTextToSize(a.value, CONTENT_WIDTH - 4);
          doc.setTextColor(50);
          doc.text(valLines, MARGIN + 4, y);
          y += valLines.length * 3 + 1;
        }
      }
      const skipped = m.assertions.length - contentAssertions.length;
      if (skipped > 0) {
        checkPage(5);
        doc.setFontSize(6.5);
        doc.setFont('helvetica', 'italic');
        doc.setTextColor(120);
        doc.text(
          `${skipped} structural hash assertion${skipped === 1 ? '' : 's'} omitted from this summary (available via raw manifest export).`,
          MARGIN + 2, y,
        );
        doc.setFont('helvetica', 'normal');
        doc.setTextColor(40);
        y += 4;
      }
    }

    // Provenance chain
    if (result.c2paChain && result.c2paChain.ingredients.length > 0) {
      const chain = result.c2paChain;
      // Sub-heading
      y += 3;
      checkPage(10);
      doc.setFontSize(9);
      doc.setFont('helvetica', 'bold');
      doc.setTextColor(60);
      const countLabel = chain.manifestCount === 1 ? 'manifest' : 'manifests';
      doc.text(`Provenance Chain (${chain.manifestCount} ${countLabel})`, MARGIN, y);
      y += 5;
      doc.setFont('helvetica', 'normal');

      // C2PA action label mapping — imported from the shared v1.4 Table 3
      // module so this file cannot drift from the verify page. See
      // ui/src/lib/c2pa-labels.ts for the full list and spec references.

      function parseActionSummary(assertions: { label: string; value: string }[]): string {
        const actionsAssertion = assertions.find(
          a => a.label === 'c2pa.actions' || a.label === 'c2pa.actions.v2'
        );
        if (!actionsAssertion) return '';
        try {
          const parsed = JSON.parse(actionsAssertion.value) as { actions?: { action?: string }[] };
          const actions = parsed?.actions ?? [];
          if (actions.length === 0) return '';
          const labels = actions
            .map(a => a.action ? (C2PA_ACTION_LABELS[a.action] ?? C2PA_UNKNOWN_ACTION_LABEL) : '')
            .filter(Boolean);
          return labels.join(', ');
        } catch {
          return '';
        }
      }

      const allNodes = [chain.active, ...chain.ingredients];
      const lastIdx = allNodes.length - 1;

      for (let idx = 0; idx < allNodes.length; idx++) {
        const node = allNodes[idx];
        // Chain-position labels per C2PA UX Rec v1.4:
        //   "Active" — §5.3 and §5.4 ("the active manifest at the top")
        //   "Origin" — §5.4 and Figure 12 ("origin ingredients … at the bottom")
        // The spec does not prescribe a label for middle manifests (it assumes
        // they are collapsed under "N additional manifests" when chain >= 4).
        // For the uncollapsed middle case we use a neutral positional label
        // rather than inventing terminology like "Intermediate".
        const nodeLabel = idx === 0 ? 'Active'
          : idx === lastIdx ? 'Origin'
          : `Step ${idx} of ${lastIdx}`;

        const signer = node.signedBy ?? node.claimGenerator ?? 'Unknown';
        const dateStr = node.signedAt
          ? new Date(node.signedAt).toLocaleString('en-GB')
          : '\u2014';
        const actionSummary = parseActionSummary(node.assertions);
        const validStatus = node.isValid
          ? (node.validAtSigning ? 'Valid at signing' : 'Valid')
          : C2PA_STATUS_INVALID;

        // Reserve enough space so a node never splits across a page break.
        // Each node uses: label (3.5) + Issued-by (up to 2 lines × 4) +
        // Date (5) + optional Action (up to 2 × 3.5) + Status (up to 2 × 4)
        // + separator (2) ≈ 30 mm worst case.
        checkPage(32);

        // Node label in bold
        doc.setFontSize(7);
        doc.setFont('helvetica', 'bold');
        doc.setTextColor(60);
        doc.text(nodeLabel, MARGIN + 2, y);
        y += 3.5;
        doc.setFont('helvetica', 'normal');

        // Local helper for wrapped label/value rows inside a chain node.
        const FIELD_X = MARGIN + 4;
        const VALUE_X = MARGIN + 24;
        const VALUE_W = CONTENT_WIDTH - (VALUE_X - MARGIN);
        function nodeRow(fieldLabel: string, valueText: string, colour?: [number, number, number]) {
          doc.setFontSize(7);
          doc.setTextColor(100);
          doc.text(fieldLabel, FIELD_X, y);
          if (colour) doc.setTextColor(colour[0], colour[1], colour[2]);
          else doc.setTextColor(40);
          const lines = doc.splitTextToSize(valueText, VALUE_W);
          doc.text(lines, VALUE_X, y);
          y += Math.max(LINE_HEIGHT, lines.length * 4);
          doc.setTextColor(40);
        }

        nodeRow('Issued by:', signer);
        nodeRow('Date:', dateStr);
        if (actionSummary) nodeRow('Action:', actionSummary);
        nodeRow(
          'Status:',
          validStatus,
          node.isValid ? [91, 138, 95] : [205, 92, 92],
        );

        // Hairline separator between nodes (not after last)
        if (idx < lastIdx) {
          doc.setDrawColor(220);
          doc.setLineWidth(0.1);
          doc.line(MARGIN + 2, y, PAGE_WIDTH - MARGIN - 2, y);
          y += 2;
        }
      }
    }

  } else {
    paragraph('No C2PA provenance manifest found in this file.');
  }
  y += SECTION_GAP;

  // ── EXIF Analysis ───────────────────────────────────────────
  if (result.exifAnalysis) {
    heading('EXIF Analysis');
    const exif = result.exifAnalysis;
    row('Trust Score', `${Math.round(exif.trustScore * 100)}%`);
    row('Fields Populated', `${exif.fieldsPopulated} / ${exif.fieldsTotal}`);
    row('EXIF Present', exif.hasExif ? 'Yes' : 'No');

    // ── Thumbnail consistency ────────────────────────────────────
    if (result.thumbnailCheck?.hasThumbnail) {
      const tc = result.thumbnailCheck;
      const outcome = tc.mismatch ? 'Mismatch detected' : 'Consistent';
      const hammingStr = tc.hammingDistance != null ? `pHash distance ${tc.hammingDistance}` : 'pHash N/A';
      const mseStr = tc.differenceScore != null ? `MSE ${tc.differenceScore.toFixed(4)}` : 'pixel comparison N/A';
      row('Thumbnail Consistency', `${outcome} (${hammingStr}, ${mseStr})`);
      if (tc.summary) {
        checkPage(LINE_HEIGHT * 2);
        doc.setFontSize(7);
        doc.setTextColor(80);
        const summaryLines = doc.splitTextToSize(tc.summary, CONTENT_WIDTH - 4);
        doc.text(summaryLines, MARGIN + 2, y);
        y += summaryLines.length * 3 + 1;
      }
    }

    if (exif.findings.length > 0) {
      y += 2;
      doc.setFontSize(8);
      doc.setTextColor(80);
      doc.text(`Findings (${exif.findings.length}):`, MARGIN, y);
      y += LINE_HEIGHT;
      for (const f of exif.findings) {
        checkPage(LINE_HEIGHT * 2);
        doc.setFontSize(8);
        doc.setTextColor(f.severity === 'critical' || f.severity === 'high' ? 180 : 60);
        doc.text(`[${f.severity.toUpperCase()}] ${f.title}`, MARGIN + 2, y);
        y += 3.5;
        doc.setTextColor(80);
        const descLines = doc.splitTextToSize(f.description, CONTENT_WIDTH - 4);
        doc.setFontSize(7);
        doc.text(descLines, MARGIN + 4, y);
        y += descLines.length * 3 + 2;
      }
    }
    y += SECTION_GAP;
  }

  // ── Forensic Analysis ───────────────────────────────────────
  const hasForensics = result.elaScore != null || result.noiseScore != null || result.copyMoveScore != null;
  if (hasForensics) {
    heading('Forensic Analysis');

    if (result.elaScore != null) {
      row('ELA Score', `${(result.elaScore * 100).toFixed(1)}%`);
    }
    if (result.noiseScore != null) {
      row('Noise Score', `${(result.noiseScore * 100).toFixed(1)}%`);
    }
    if (result.copyMoveScore != null) {
      row('Copy-Move Score', `${(result.copyMoveScore * 100).toFixed(1)}%`);
    }

    // Embed heatmap images (paths resolved via Tauri asset protocol)
    const heatmaps: { label: string; data?: string }[] = [
      { label: 'ELA Heatmap', data: result.elaResult?.elaImageUrl },
      { label: 'Noise Heatmap', data: result.noiseResult?.heatmapUrl },
      { label: 'Copy-Move Visualisation', data: result.copyMoveResult?.visualisationUrl },
      { label: 'AI Detection Heatmap', data: result.deepfakeResult?.heatmapUrl },
    ];

    for (const hm of heatmaps) {
      if (!hm.data) continue;
      checkPage(65);
      doc.setFontSize(8);
      doc.setTextColor(80);
      doc.text(hm.label, MARGIN, y);
      y += 4;
      try {
        const jpegDataUrl = await transcodePngToJpeg(hm.data);
        doc.addImage(jpegDataUrl, 'JPEG', MARGIN, y, CONTENT_WIDTH * 0.7, 55, undefined, 'FAST');
        y += 58;
      } catch {
        doc.setFontSize(7);
        doc.setTextColor(120);
        doc.text('(Image could not be embedded)', MARGIN, y);
        y += LINE_HEIGHT;
      }
    }
    y += SECTION_GAP;
  }

  // ── AI Generation Detection ─────────────────────────────────
  if (result.deepfakeResult) {
    heading('AI Generation Detection');
    const df = result.deepfakeResult;
    row('Score', `${(df.score * 100).toFixed(1)}%`);
    row('Confidence', df.confidence);
    row('Suspicious', df.suspicious ? 'Yes' : 'No');

    if (df.watermarks && df.watermarks.length > 0) {
      y += 2;
      doc.setFontSize(8);
      doc.setTextColor(80);
      doc.text('Watermarks detected:', MARGIN, y);
      y += LINE_HEIGHT;
      for (const wm of df.watermarks) {
        checkPage(LINE_HEIGHT);
        row(wm.watermarkType, `${wm.detected ? 'Detected' : 'Not detected'} (${(wm.confidence * 100).toFixed(0)}%)`);
      }
    }

    if (df.signals.length > 0) {
      y += 2;
      doc.setFontSize(8);
      doc.setTextColor(80);
      doc.text(`Signals (${df.signals.filter(s => s.triggered).length} of ${df.signals.length} triggered):`, MARGIN, y);
      y += LINE_HEIGHT;
      for (const s of df.signals) {
        checkPage(LINE_HEIGHT);
        doc.setFontSize(7);
        doc.setTextColor(s.triggered ? 160 : 100);
        doc.text(
          `${s.triggered ? '+' : '-'} ${s.name} (weight: ${s.weight.toFixed(1)})`,
          MARGIN + 2, y
        );
        y += 3.5;
      }
    }
    y += SECTION_GAP;
  }

  // Video Frame Analysis section removed — videoFramesResult not
  // populated by backend. Video deepfake per-frame scores still render
  // in the Video Deepfake section above.

  // Compute once here — used both in the signal-scores section below
  // and in the Methodology block further down.
  const hasAuthoritativeList = !!(result.detectorsRun && result.detectorsRun.length > 0);

  // ── Forensic Signal Scores ───────────────────────────────────
  // Determine which primary and regional detector results are present
  const hasAnySignalScores =
    result.elaResult != null ||
    result.noiseResult != null ||
    result.copyMoveResult != null ||
    result.deepfakeResult != null ||
    result.nprResult != null ||
    result.jpegGhostResult != null;

  const hasRegionalScores =
    result.segmentedElaResult != null ||
    result.shadowConsistencyResult != null ||
    result.colourTemperatureResult != null ||
    result.spliceBoundaryResult != null;

  // When we have the authoritative detectors-run list, also show rows for
  // any detector that was expected but did not run (labelled explicitly).
  const detRunSet = hasAuthoritativeList
    ? new Set(result.detectorsRun!)
    : null;
  const expDetectors = hasAuthoritativeList
    ? new Set(expectedDetectors(result.mode, result.contentType))
    : new Set<string>();

  // A detector row should appear if: (a) the result field is populated, OR
  // (b) it is expected for the mode/content-type but absent from detectorsRun.
  function shouldShowRow(id: string, resultPresent: boolean): boolean {
    if (resultPresent) return true;
    if (detRunSet && expDetectors.has(id) && !detRunSet.has(id)) return true;
    return false;
  }

  function isNotRun(id: string, resultPresent: boolean): boolean {
    if (!detRunSet) return false; // legacy — never force not-run label
    return expDetectors.has(id) && !detRunSet.has(id) && !resultPresent;
  }

  const showEla       = shouldShowRow('ela',               result.elaResult != null);
  const showNoise     = shouldShowRow('noise',             result.noiseResult != null);
  const showCopyMove  = shouldShowRow('copy_move',         result.copyMoveResult != null);
  const showDeepfake  = shouldShowRow('deepfake',          result.deepfakeResult != null);
  const showNpr       = shouldShowRow('npr',               result.nprResult != null);
  const showJpegGhost = shouldShowRow('jpeg_ghost',        result.jpegGhostResult != null);
  const showSegEla    = shouldShowRow('segmented_ela',     result.segmentedElaResult != null);
  const showShadow    = shouldShowRow('shadow_consistency',result.shadowConsistencyResult != null);
  const showColTemp   = shouldShowRow('colour_temperature',result.colourTemperatureResult != null);
  const showSplice    = shouldShowRow('splice_boundary',   result.spliceBoundaryResult != null);

  const hasPrimaryRows  = showEla || showNoise || showCopyMove || showDeepfake || showNpr || showJpegGhost;
  const hasRegionalRows = showSegEla || showShadow || showColTemp || showSplice;

  if (hasPrimaryRows || hasRegionalRows || hasAnySignalScores || hasRegionalScores) {
    heading('Forensic Signal Scores');

    paragraph(
      'Raw numerical scores from each detector (range 0.00\u20131.00, normalised). ' +
      'A score at or above the listed threshold triggers a \u201cFlagged\u201d status. ' +
      'Detectors without a fixed published threshold are marked \u2014 and use their own internal suspicious flag instead. ' +
      '\u201cNot run in this analysis\u201d indicates the detector was expected for this mode and content type but did not produce a result.',
      7
    );
    y += 2;
  }

  if (hasPrimaryRows || hasAnySignalScores) {
    signalTableHeader();

    if (showEla) {
      signalTableRow(
        'Error Level Analysis (ELA)',
        result.elaResult?.score,
        DETECTOR_THRESHOLDS.ela,
        result.elaResult?.suspicious,
        isNotRun('ela', result.elaResult != null)
      );
    }
    if (showNoise) {
      signalTableRow(
        'Noise Analysis',
        result.noiseResult?.score,
        DETECTOR_THRESHOLDS.noise,
        result.noiseResult?.suspicious,
        isNotRun('noise', result.noiseResult != null)
      );
    }
    if (showCopyMove) {
      signalTableRow(
        'Copy-Move Detection',
        result.copyMoveResult?.score,
        DETECTOR_THRESHOLDS.copyMove,
        result.copyMoveResult?.suspicious,
        isNotRun('copy_move', result.copyMoveResult != null)
      );
    }
    if (showDeepfake) {
      signalTableRow(
        'AI Generation (Deepfake)',
        result.deepfakeResult?.score,
        DETECTOR_THRESHOLDS.deepfake,
        result.deepfakeResult?.suspicious,
        isNotRun('deepfake', result.deepfakeResult != null)
      );
    }
    if (showNpr) {
      signalTableRow(
        'Neighbouring Pixel Relationships',
        result.nprResult?.score,
        DETECTOR_THRESHOLDS.npr,
        result.nprResult?.suspicious,
        isNotRun('npr', result.nprResult != null)
      );
    }
    if (showJpegGhost) {
      signalTableRow(
        'JPEG Ghost',
        result.jpegGhostResult?.score,
        DETECTOR_THRESHOLDS.jpegGhost,
        result.jpegGhostResult?.suspicious,
        isNotRun('jpeg_ghost', result.jpegGhostResult != null)
      );
      // Experimental weight note — ADR DEC-2026-04-09-001 / backlog item #11.
      // The 0.5× weight is a cross-review consensus value, not empirically
      // calibrated (S28-FU9: synthetic corpus could not exercise detector).
      checkPage(5);
      doc.setFontSize(6.5);
      doc.setFont('helvetica', 'italic');
      doc.setTextColor(120);
      doc.text(
        'Experimental weight (0.5\u00D7) \u2014 calibration tracked against a commercial-cleared ' +
        'splice benchmark. See methodology disclosure for current calibration state.',
        COL_DETECTOR + 2, y
      );
      doc.setFont('helvetica', 'normal');
      doc.setTextColor(40);
      y += 4;
    }

    y += 2;

    // Legacy footer — shown only when detectorsRun is absent, so we cannot
    // distinguish "ran and returned null" from "never ran in this mode".
    if (!hasAuthoritativeList) {
      checkPage(8);
      doc.setFontSize(7);
      doc.setFont('helvetica', 'italic');
      doc.setTextColor(120);
      const legacyNote = 'Legacy verification \u2014 detector lineup inferred from result fields. ' +
        'Regenerate in this build for an authoritative list.';
      const legacyLines = doc.splitTextToSize(legacyNote, CONTENT_WIDTH);
      doc.text(legacyLines, MARGIN, y);
      y += legacyLines.length * 3 + 2;
      doc.setFont('helvetica', 'normal');
      doc.setTextColor(40);
    }
  }

  // Regional Analysis subsection (deep mode only; legacy archival aliased to deep)
  if (hasRegionalRows || hasRegionalScores) {
    checkPage(12);
    doc.setFontSize(9);
    doc.setFont('helvetica', 'bold');
    doc.setTextColor(60);
    doc.text('Regional Analysis Scores', MARGIN, y);
    y += 5;
    doc.setFont('helvetica', 'normal');

    signalTableHeader();

    if (showSegEla) {
      signalTableRow(
        'Segmented ELA',
        result.segmentedElaResult?.score,
        null,
        result.segmentedElaResult?.suspicious,
        isNotRun('segmented_ela', result.segmentedElaResult != null)
      );
    }
    if (showShadow) {
      signalTableRow(
        'Shadow Consistency',
        result.shadowConsistencyResult?.score,
        null,
        result.shadowConsistencyResult?.suspicious,
        isNotRun('shadow_consistency', result.shadowConsistencyResult != null)
      );
    }
    if (showColTemp) {
      signalTableRow(
        'Colour Temperature',
        result.colourTemperatureResult?.score,
        null,
        result.colourTemperatureResult?.suspicious,
        isNotRun('colour_temperature', result.colourTemperatureResult != null)
      );
    }
    if (showSplice) {
      signalTableRow(
        'Splice Boundary',
        result.spliceBoundaryResult?.score,
        null,
        result.spliceBoundaryResult?.suspicious,
        isNotRun('splice_boundary', result.spliceBoundaryResult != null)
      );
    }

    y += 2;
  }

  if (hasPrimaryRows || hasRegionalRows || hasAnySignalScores || hasRegionalScores) {
    y += SECTION_GAP;
  }

  // ── Methodology ─────────────────────────────────────────────
  heading('Methodology');

  // Structured pipeline metadata block. 'archival' was retired
  // 2026-04-22 and aliased to 'deep' in the Rust backend; render
  // legacy records as "Deep" rather than naming a mode the current
  // build no longer offers.
  const analysisMode = result.mode ?? 'standard';
  const modeLabel =
    analysisMode === 'archival' ? 'Deep' :
    analysisMode === 'deep' ? 'Deep' :
    'Standard';

  // Build the list of detectors that actually ran.
  //
  // Prefer the authoritative `result.detectorsRun` list provided by the
  // Rust backend (Sprint 28 S28-FU1 — matches the schema v6 `detectors_run`
  // DB column). This avoids the previous "infer from *Result field
  // population" approach that couldn't distinguish "ran and returned null"
  // from "never ran in this build / mode". Fall back to the legacy
  // inference path when an older sidecar version (pre-FU1) returns a
  // result without the new field, so this file stays compatible with
  // case exports loaded from older DB rows.
  // (hasAuthoritativeList is declared earlier, before the signal-scores section.)
  let detectorsRun: string[];
  if (hasAuthoritativeList) {
    detectorsRun = result.detectorsRun!.map(
      (id) => DETECTOR_ID_LABELS[id] ?? id,
    );
  } else {
    // Legacy inference path for pre-S28-FU1 results.
    detectorsRun = [];
    if (result.elaResult) detectorsRun.push('ELA');
    if (result.noiseResult) detectorsRun.push('Noise Analysis');
    if (result.copyMoveResult) detectorsRun.push('Copy-Move Detection');
    if (result.deepfakeResult) detectorsRun.push('AI Generation Detection');
    if (result.nprResult) detectorsRun.push('Neighbouring Pixel Relationships');
    if (result.jpegGhostResult) detectorsRun.push('JPEG Ghost');
    if (result.segmentedElaResult) detectorsRun.push('Segmented ELA');
    if (result.shadowConsistencyResult) detectorsRun.push('Shadow Consistency');
    if (result.colourTemperatureResult) detectorsRun.push('Colour Temperature');
    if (result.spliceBoundaryResult) detectorsRun.push('Splice Boundary');
    if (result.clipResult) detectorsRun.push('CLIP Detection');
    if (result.exifAnalysis) detectorsRun.push('EXIF Anomaly Analysis');
    if (result.c2paManifest !== undefined) detectorsRun.push('C2PA Credential Verification');
  }

  const detectorsRunText = detectorsRun.length > 0 ? detectorsRun.join(', ') : 'None recorded';

  // Model version labels
  const classifierModel = result.deepfakeResult?.classifierAvailable
    ? 'GBM v4 (AUC 0.9868) + UnivFD v10onnx (AUC 0.9929) ensemble'
    : 'Heuristic only';
  const clipModel = result.clipResult
    ? 'ViT-B/32 (open_clip)'
    : 'Not available';

  // Use MethodologyRecord from the backend when available, fall back to
  // locally-computed values for results from older pipeline versions.
  const meth = result.methodology;
  const pipelineVer = meth?.pipelineVersion ?? version;
  const sidecarVer = meth?.sidecarVersion ?? null;
  const classifierHash = meth?.classifierModelHash
    ? meth.classifierModelHash.substring(0, 12) + '…'
    : null;
  const analysedAtIso = meth?.analysedAt
    ? new Date(meth.analysedAt).toISOString()
    : new Date(meta.analysedAt).toISOString();

  // Analysis-completeness label — explicit X-of-Y disclosure for solicitor /
  // audit-reviewer use. The Partial Analysis Notice banner above carries the
  // prominent warning; this row carries the structured value for traceability.
  const completenessLabel = expectedCount > 0
    ? `${actualCount} of ${expectedCount} expected${isPartialAnalysis ? ' — PARTIAL' : ''}`
    : `${actualCount} (expected count not derivable for this content type)`;

  // Trust-formula row updated 2026-05-22 after the doc-drift audit. The
  // earlier "40% EXIF + 60% forensic" string was a Berkeley Protocol §6
  // reproducibility risk: the code uses 20/80 (reduced from 40/60 after a
  // security audit). The new copy walks the actual algorithm shape so a
  // reviewer with the open-source AGPL source can trace any number printed
  // in this PDF back to the function that produced it. Full breakdown:
  // src-tauri/src/verify/trust.rs::compute_trust.
  const trustFormulaText =
    'Forensic signal analysis (primary, worst-case across pixel-level detectors). ' +
    'EXIF metadata consistency (corroborating, capped at 20% weight). ' +
    'C2PA provenance adjustment (+0.10 valid manifest, -0.25 self-declared AI). ' +
    'Composite-evidence cap at 0.55 when two regional detectors agree. ' +
    'Deepfake verdict ceiling (synthetic-high 0.25, synthetic-medium 0.35, ' +
    'synthetic-low 0.45, inconclusive 0.55). Full algorithm published under ' +
    'AGPL-3.0 in src-tauri/src/verify/trust.rs::compute_trust.';

  const metaRows: [string, string][] = [
    ['Jura Trace version', `v${pipelineVer}`],
    ...(sidecarVer ? [['Analysis Engine version', sidecarVer] as [string, string]] : []),
    ['Analysis mode', modeLabel],
    ['Analysis completeness', completenessLabel],
    ['Trust formula', trustFormulaText],
    ['C2PA Content Credentials', 'Valid manifest: +0.10 trust signal. AI disclosure (DigitalSourceType) surfaced separately at L2/L3 — honest disclosure is not penalised.'],
    ['Classifier model', classifierModel + (classifierHash ? ` (${classifierHash})` : '')],
    ['CLIP model', clipModel],
    ['Detectors run', detectorsRunText],
    ['Analysis date', analysedAtIso],
  ];

  // Estimate height needed: 6 rows × 5 mm plus 2 mm padding each side
  checkPage(metaRows.length * 6 + 6);

  // Light box around the metadata block
  const boxStartY = y - 2;
  // Render rows first so we know the actual height consumed
  const boxContentStartY = y;
  doc.setDrawColor(200);
  doc.setLineWidth(0.2);

  for (const [k, v] of metaRows) {
    checkPage(LINE_HEIGHT);
    doc.setFont('helvetica', 'bold');
    doc.setFontSize(7);
    doc.setTextColor(80);
    doc.text(`${k}:`, MARGIN + 2, y);
    doc.setFont('helvetica', 'normal');
    doc.setTextColor(40);
    // Word-wrap long values (the detectors list in particular can overflow)
    const valueLines = doc.splitTextToSize(v, CONTENT_WIDTH - 42);
    doc.text(valueLines, MARGIN + 40, y);
    y += valueLines.length > 1 ? valueLines.length * 4 : LINE_HEIGHT;
  }

  y += 3;
  // Now draw the box retrospectively around the content we just rendered
  doc.rect(MARGIN, boxStartY, CONTENT_WIDTH, y - boxContentStartY + 4);

  // Inline disclaimer
  y += 3;
  checkPage(10);
  doc.setFontSize(7);
  doc.setFont('helvetica', 'italic');
  doc.setTextColor(100);
  const inlineDisclaimer =
    'This report was generated by Jura Trace, a local-first forensic verification tool. ' +
    'Results are probabilistic indicators, not legal proof. ' +
    'See juralabs.org/methodology for full documentation.';
  const idLines = doc.splitTextToSize(inlineDisclaimer, CONTENT_WIDTH);
  doc.text(idLines, MARGIN, y);
  y += idLines.length * 3 + 2;
  doc.setFont('helvetica', 'normal');

  y += SECTION_GAP;

  // ── Methodology Disclosure (detector descriptions with citations) ────────
  heading('Methodology Disclosure');

  // Each entry pairs a description with an optional citation key from DETECTOR_CITATIONS.
  const methodologyEntries: { text: string; citationKey?: string }[] = [
    {
      text: 'Error Level Analysis (ELA): Recompresses the image at a fixed JPEG quality and measures pixel-level differences. Regions with inconsistent compression artefacts may indicate editing.',
      citationKey: 'ela',
    },
    {
      text: 'Noise Analysis: Divides the image into blocks and measures variance in each. Inconsistent noise patterns across blocks can indicate splicing or inpainting.',
      citationKey: 'noise',
    },
    {
      text: 'Copy-Move Detection: Uses SIFT feature matching with RANSAC geometric verification to find duplicated regions within the image. Clustered, geometrically consistent matches suggest content has been cloned from one area to another.',
      citationKey: 'copyMove',
    },
    {
      text: 'AI Generation Detection: A two-head ensemble — GBM v4 (84-feature gradient-boosted classifier on hand-engineered forensic features, AUC 0.9868) and UnivFD v10onnx (logistic regression on CLIP ViT-B/32 embeddings, AUC 0.9929, multi-format augmentation across PNG/TIFF/WebP/HEIC). Each head runs independently and the verdict reflects their combined output.',
      citationKey: 'deepfake',
    },
    {
      text: 'Neighbouring Pixel Relationships (NPR): Analyses statistical correlations between adjacent pixels in horizontal, vertical, and diagonal directions. AI-generated images often exhibit atypical inter-pixel dependencies.',
      citationKey: 'npr',
    },
    {
      text: 'JPEG Ghost: Recompresses the image across multiple JPEG quality levels and identifies regions that deviate significantly from a consistent quality history, suggesting prior manipulation.',
      citationKey: 'jpegGhost',
    },
    {
      text: 'Segmented ELA: Divides the image into an 8x8 grid and computes per-region ELA scores. High inter-region variance suggests inconsistent editing or compositing.',
      citationKey: 'segmentedEla',
    },
    {
      text: 'Shadow Consistency: Estimates the dominant light direction in each image region using gradient analysis. Significant directional inconsistencies between regions suggest compositing.',
      citationKey: 'shadowConsistency',
    },
    {
      text: 'Colour Temperature: Segments the image in CIELAB colour space and measures per-region colour temperature. Abrupt temperature changes across regions can indicate splicing.',
      citationKey: 'colourTemperature',
    },
    {
      text: 'Splice Boundary: Applies three complementary edge detectors (JPEG grid alignment, noise asymmetry, feathering patterns) at grid junctions to locate compositing boundaries.',
      citationKey: 'spliceBoundary',
    },
    {
      text: 'EXIF Anomaly Analysis: Checks embedded metadata for consistency, completeness, and known manipulation patterns. Missing or contradictory metadata reduces trust.',
      citationKey: 'exifAnomaly',
    },
    {
      text: 'C2PA Provenance: Verifies cryptographically signed provenance manifests embedded in the file, following the Coalition for Content Provenance and Authenticity specification.',
      citationKey: 'c2pa',
    },
    {
      text: "All analysis is performed locally on the user\u2019s device. No data is transmitted to external servers at any point during the verification process.",
    },
  ];

  for (const entry of methodologyEntries) {
    paragraph(entry.text, 7);
    if (entry.citationKey) {
      const cit = DETECTOR_CITATIONS[entry.citationKey];
      if (cit) {
        checkPage(5);
        doc.setFontSize(6.5);
        doc.setFont('helvetica', 'italic');
        doc.setTextColor(120);
        const citText = `${cit.prefix ? `${cit.prefix}: ` : ''}${cit.authors} (${cit.year}). ${cit.paper}.`;
        const citLines = doc.splitTextToSize(citText, CONTENT_WIDTH - 4);
        doc.text(citLines, MARGIN + 4, y);
        y += citLines.length * 3 + 1;
        doc.setFont('helvetica', 'normal');
        doc.setTextColor(60);
      }
    }
    y += 1;
  }

  // ── References ──────────────────────────────────────────────────────────
  // Full bibliography of all cited works, sorted alphabetically by first author surname.
  y += SECTION_GAP;
  heading('References');

  // Build sorted bibliography from all entries that carry a citation.
  const citedKeys = methodologyEntries
    .filter(e => e.citationKey)
    .map(e => e.citationKey as string);
  // Deduplicate by citation content, not key: distinct detectors may share a
  // reference (e.g. ELA and Segmented ELA both cite Krawetz 2007) and must
  // not produce duplicate bibliography entries.
  const uniqueKeys = [...new Set(citedKeys)];
  const seenWorks = new Set<string>();
  const uniqueWorks = uniqueKeys
    .map(k => ({ key: k, cit: DETECTOR_CITATIONS[k] }))
    .filter(e => !!e.cit)
    .filter(e => {
      const id = `${e.cit.authors}|${e.cit.year}|${e.cit.paper}`;
      if (seenWorks.has(id)) return false;
      seenWorks.add(id);
      return true;
    });

  // Sort by author surname (first word before comma or space).
  const sorted = uniqueWorks
    .sort((a, b) => {
      const surnameA = a.cit.authors.split(/[,\s]/)[0].toLowerCase();
      const surnameB = b.cit.authors.split(/[,\s]/)[0].toLowerCase();
      if (surnameA < surnameB) return -1;
      if (surnameA > surnameB) return 1;
      return a.cit.year - b.cit.year;
    });

  // Number sequentially for easy cross-referencing.
  for (let i = 0; i < sorted.length; i++) {
    const { cit } = sorted[i];
    const refText = `[${i + 1}] ${cit.authors} (${cit.year}). ${cit.paper}.`;
    checkPage(8);
    doc.setFontSize(7);
    doc.setFont('helvetica', 'normal');
    doc.setTextColor(50);
    const refLines = doc.splitTextToSize(refText, CONTENT_WIDTH);
    doc.text(refLines, MARGIN, y);
    y += refLines.length * 3.5 + 1;
  }

  // ── Disclaimer ──────────────────────────────────────────────
  y += SECTION_GAP;
  checkPage(15);
  doc.setFontSize(7);
  doc.setTextColor(120);
  const disclaimer = 'This report is generated by automated analysis tools and should be interpreted by qualified professionals. Results are indicative, not conclusive. To the maximum extent permitted by law, and consistent with the AGPL-3.0-or-later "no warranty" provisions, Jura Labs CIC excludes liability for indirect, consequential, or special losses arising from decisions made based on this report. Liability for death, personal injury, fraud or other matters that cannot lawfully be excluded is not affected.';
  const disclaimerLines = doc.splitTextToSize(disclaimer, CONTENT_WIDTH);
  doc.text(disclaimerLines, MARGIN, y);
  y += disclaimerLines.length * 3 + 2;

  // ── Berkeley: Known Limitations ───────────────────────────
  if (reportFormat === 'berkeley') {
    y += SECTION_GAP;
    heading('Known Limitations');
    const limitations = [
      '\u2022 JPEG recompression: Repeated JPEG saving at different quality levels can introduce artefacts that mimic manipulation. ELA and JPEG Ghost detectors are particularly sensitive to this.',
      '\u2022 Screenshots and re-encoded media: Screen captures, social media re-uploads, and messaging app compression destroy forensic signals, reducing detector reliability.',
      '\u2022 Absence of C2PA credentials: Many legitimate images lack C2PA provenance data. The absence of credentials does not indicate inauthenticity.',
      '\u2022 Probabilistic scores: All detector outputs are statistical estimates, not binary determinations. Scores near thresholds should be interpreted with caution and corroborated by other evidence.',
      '\u2022 Regional detector false flags: Segmented ELA, shadow consistency, colour temperature, and splice boundary detectors can produce false positives on images with natural lighting variation, complex scenes, or intentional artistic editing.',
    ];
    for (const text of limitations) {
      paragraph(text, 7);
      y += 1;
    }

    // ── Berkeley: Formal Analyst Declaration ──────────────────
    y += SECTION_GAP;
    heading('Analyst Declaration');

    doc.setDrawColor(60);
    doc.setLineWidth(0.3);
    doc.line(MARGIN, y, PAGE_WIDTH - MARGIN, y);
    y += 5;

    const declarantName = ctx?.analystName?.trim() || '[Name not provided]';
    const declarantOrg = ctx?.organisation?.trim() || '[Organisation not provided]';

    paragraph(
      `I, ${declarantName}, of ${declarantOrg}, declare that:`,
      8
    );
    y += 2;

    const declarations = [
      '1. The analysis documented in this report was conducted using the methodology described herein, following the principles of the Berkeley Protocol on Digital Open Source Investigations (2020).',
      '2. All processing was performed locally on my device using Jura Trace. No copy of the evidential material was transmitted to any external server, cloud service, or third party during the analysis process.',
      '3. The scores and verdicts presented are generated by automated detection algorithms. They represent probabilistic assessments and should not be treated as conclusive determinations of authenticity or manipulation.',
      '4. I am aware of the known limitations of these tools as documented in this report, and I have taken these limitations into account in forming any opinions expressed in the analyst notes.',
    ];

    for (const decl of declarations) {
      paragraph(decl, 7);
      y += 2;
    }

    y += 4;
    // Signature lines
    doc.setFontSize(8);
    doc.setTextColor(80);
    doc.text('Signed: ___________________________________', MARGIN, y);
    y += LINE_HEIGHT + 2;
    doc.text('Date: ___________________________________', MARGIN, y);
    y += LINE_HEIGHT;

    doc.setDrawColor(60);
    doc.setLineWidth(0.3);
    doc.line(MARGIN, y + 2, PAGE_WIDTH - MARGIN, y + 2);

    // ── Berkeley: Appendix — Tool Versions ────────────────────
    y += SECTION_GAP + 4;
    heading('Appendix: Tool Versions');

    // Berkeley Protocol requires reproducible identifiers, not marketing version
    // strings. The classifier is identified by the SHA-256 hash of the model file
    // captured in MethodologyRecord at verification time — a legally defensible
    // fingerprint that cannot go stale as models are retrained.
    const berkeleyClassifier = result.deepfakeResult?.classifierAvailable
      ? result.methodology?.classifierModelHash
        ? `GBM classifier (SHA-256: ${result.methodology.classifierModelHash.substring(0, 16)}…)`
        : 'GBM classifier (hash unavailable)'
      : 'Heuristic only';
    const berkeleyClip = result.clipResult
      ? 'ViT-B/32 (open_clip)'
      : 'Not available';

    const toolVersions: [string, string][] = [
      ['Jura Trace', `v${version}`],
      ['Report format', 'Berkeley Protocol (2020)'],
      ['Deepfake classifier', berkeleyClassifier],
      ['CLIP model', berkeleyClip],
      ['C2PA library', 'c2pa-rs (Rust)'],
      ['Perceptual hashing', 'image_hasher (aHash, dHash, pHash)'],
      ['ML sidecar', 'Python 3.13 + FastAPI'],
      ['Report generator', 'jsPDF (client-side)'],
    ];

    for (const [k, v] of toolVersions) {
      checkPage(LINE_HEIGHT);
      label(k);
      value(v);
    }
  }

  addFooter();

  return doc.output('blob');
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
