// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, it, expect } from 'vitest';
import JSZip from 'jszip';
import {
  packVerificationRecord,
  verificationRecordFileName,
  RECORD_FILE,
  SIGNATURE_FILE,
  CERTIFICATE_FILE,
  INSTRUCTIONS_FILE,
  type VerificationRecordExport,
} from './verificationRecord';

const sample: VerificationRecordExport = {
  // Non-ASCII and a trailing newline on purpose: both are places where a
  // string round trip could change the bytes the signature covers.
  recordJson: '{\n  "subject": { "fileName": "café – 写真.jpg" }\n}\n',
  signatureDer: [0x30, 0x45, 0x02, 0x21, 0x00, 0xff, 0x80, 0x0a, 0x0d],
  certificatePem: '-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n',
  instructions: 'how to check\n',
  recordSha256: 'ab',
  certificateSha256: 'AA:BB',
};

describe('packVerificationRecord', () => {
  it('writes record.json byte for byte, so the signature still covers it', async () => {
    const zip = await JSZip.loadAsync(await (await packVerificationRecord(sample)).arrayBuffer());
    const bytes = await zip.file(RECORD_FILE)!.async('uint8array');
    expect(Array.from(bytes)).toEqual(Array.from(new TextEncoder().encode(sample.recordJson)));
  });

  it('writes the signature as raw bytes, not as text', async () => {
    const zip = await JSZip.loadAsync(await (await packVerificationRecord(sample)).arrayBuffer());
    const bytes = await zip.file(SIGNATURE_FILE)!.async('uint8array');
    expect(Array.from(bytes)).toEqual(sample.signatureDer);
  });

  it('carries exactly the four files the instructions name', async () => {
    const zip = await JSZip.loadAsync(await (await packVerificationRecord(sample)).arrayBuffer());
    expect(Object.keys(zip.files).sort()).toEqual(
      [RECORD_FILE, SIGNATURE_FILE, CERTIFICATE_FILE, INSTRUCTIONS_FILE].sort(),
    );
  });
});

describe('verificationRecordFileName', () => {
  it('keeps the name filesystem-safe', () => {
    expect(verificationRecordFileName('my photo (1).jpg', 1700000000)).toBe(
      'jura-verification-record-my_photo__1_.jpg-1700000000.zip',
    );
    expect(verificationRecordFileName(null, 1)).toBe('jura-verification-record-file-1.zip');
  });
});
