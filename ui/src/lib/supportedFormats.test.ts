// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Tests for the format gate shared by the file picker and both
 * drag-and-drop paths on the Verify page.
 *
 * Run with: cd ui && npm test
 */
import { describe, expect, it } from 'vitest';
import { SUPPORTED_EXTENSIONS, isSupportedFile } from './supportedFormats';

describe('isSupportedFile — accepts what v1.0 verifies', () => {
  it('accepts every extension in the supported list', () => {
    for (const ext of SUPPORTED_EXTENSIONS) {
      expect(isSupportedFile(`photo.${ext}`)).toBe(true);
    }
  });

  it('is case insensitive, because file systems are not consistent', () => {
    expect(isSupportedFile('PHOTO.JPG')).toBe(true);
    expect(isSupportedFile('photo.JpEg')).toBe(true);
    expect(isSupportedFile('IMG_0001.HEIC')).toBe(true);
  });

  it('handles full paths on both separators', () => {
    expect(isSupportedFile('/Users/paul/Pictures/evidence.png')).toBe(true);
    expect(isSupportedFile('C:\\Users\\paul\\Pictures\\evidence.png')).toBe(true);
  });
});

describe('isSupportedFile — rejects what v1.0 does not verify', () => {
  // The case that matters. A dropped PDF used to run the whole pipeline
  // while the help page said PDFs were excluded, and lopdf 0.34 has a known
  // stack overflow on deeply nested documents (RUSTSEC-2026-0187).
  it('rejects PDFs', () => {
    expect(isSupportedFile('report.pdf')).toBe(false);
    expect(isSupportedFile('/tmp/hostile.PDF')).toBe(false);
  });

  it('rejects video and audio, which were dropped under JTV-138', () => {
    for (const name of ['clip.mp4', 'clip.mov', 'clip.mkv', 'take.wav', 'take.mp3']) {
      expect(isSupportedFile(name)).toBe(false);
    }
  });

  it('rejects office documents and archives', () => {
    for (const name of ['notes.docx', 'sheet.xlsx', 'bundle.zip', 'proof.zip']) {
      expect(isSupportedFile(name)).toBe(false);
    }
  });

  it('rejects executables, which a drop handler must never pass on', () => {
    for (const name of ['setup.exe', 'installer.msi', 'script.sh', 'payload.dll']) {
      expect(isSupportedFile(name)).toBe(false);
    }
  });

  it('rejects names with no extension at all', () => {
    expect(isSupportedFile('README')).toBe(false);
    expect(isSupportedFile('')).toBe(false);
    expect(isSupportedFile('/tmp/no-extension-here')).toBe(false);
  });

  it('rejects a dotfile with no real extension', () => {
    expect(isSupportedFile('.gitignore')).toBe(false);
    expect(isSupportedFile('/home/paul/.bashrc')).toBe(false);
  });

  it('judges by the final extension, not an earlier one', () => {
    // A file called photo.png.pdf is a PDF.
    expect(isSupportedFile('photo.png.pdf')).toBe(false);
    // And archive.zip.png is, as far as this gate is concerned, a PNG. The
    // Rust format router sniffs the real content afterwards; this gate only
    // decides which pipeline the file is offered to.
    expect(isSupportedFile('archive.zip.png')).toBe(true);
  });
});
