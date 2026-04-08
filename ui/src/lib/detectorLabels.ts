/**
 * Stable detector ID vocabulary and display-name labels.
 *
 * IDs are permanent and must not change across builds — they are stored in
 * the SQLite `detectors_run` column (schema v6) and embedded in ZIP case
 * exports. Labels may change but IDs are the canonical reference.
 *
 * Vocabulary mirrors `src-tauri/src/lib.rs` `detectors_run_list`.
 */
export const DETECTOR_ID_LABELS: Record<string, string> = {
  exif_anomaly:       'EXIF Anomaly Analysis',
  c2pa:               'C2PA Credential Verification',
  ela:                'ELA',
  noise:              'Noise Analysis',
  copy_move:          'Copy-Move Detection',
  deepfake:           'AI Generation Detection',
  jpeg_ghost:         'JPEG Ghost',
  segmented_ela:      'Segmented ELA',
  colour_temperature: 'Colour Temperature',
  clip:               'CLIP Detection',
  watermark:          'Watermark Extraction',
  video_deepfake:     'Video Deepfake Analysis',
  transcription:      'Audio/Video Transcription',
  // On-demand detectors — only populated when the user manually triggers them
  npr:                'Neighbouring Pixel Relationships (on-demand)',
  shadow_consistency: 'Shadow Consistency (on-demand)',
  splice_boundary:    'Splice Boundary (on-demand)',
};
