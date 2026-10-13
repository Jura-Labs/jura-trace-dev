// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Packs a Verification Record into the ZIP a user hands to somebody else.
 *
 * The record is built and signed in Rust (src-tauri/src/verification_record.rs),
 * because the signing key never reaches the webview. All this module does is
 * put the four files in an archive, and the one thing it must not do is alter
 * a byte of record.json: the signature is over those exact bytes. So the text
 * is encoded here, once, and handed to JSZip as bytes rather than as a string.
 */

import JSZip from 'jszip';

/** Mirrors `VerificationRecordExport` in src-tauri/src/verification_record.rs. */
export interface VerificationRecordExport {
  recordJson: string;
  signatureDer: number[];
  certificatePem: string;
  instructions: string;
  recordSha256: string;
  certificateSha256: string;
}

/** File names inside the archive. HOW-TO-CHECK.txt names them, so they are fixed. */
export const RECORD_FILE = 'record.json';
export const SIGNATURE_FILE = 'record.json.sig';
export const CERTIFICATE_FILE = 'signer-cert.pem';
export const INSTRUCTIONS_FILE = 'HOW-TO-CHECK.txt';

export async function packVerificationRecord(ex: VerificationRecordExport): Promise<Blob> {
  const zip = new JSZip();
  const text = new TextEncoder();
  zip.file(RECORD_FILE, text.encode(ex.recordJson));
  zip.file(SIGNATURE_FILE, new Uint8Array(ex.signatureDer));
  zip.file(CERTIFICATE_FILE, text.encode(ex.certificatePem));
  zip.file(INSTRUCTIONS_FILE, text.encode(ex.instructions));
  return zip.generateAsync({ type: 'blob' });
}

export function verificationRecordFileName(fileName: string | null | undefined, unixSeconds: number): string {
  const safe = (fileName ?? 'file').replace(/[^a-zA-Z0-9._-]/g, '_');
  return `jura-verification-record-${safe}-${unixSeconds}.zip`;
}
