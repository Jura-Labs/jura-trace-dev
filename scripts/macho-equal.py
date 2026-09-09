#!/usr/bin/env python3
"""
macho-equal.py — are two 64-bit Mach-O files the same code?

    python3 scripts/macho-equal.py A B      exit 0 if equal, 1 if not, 2 on error

Compares the two files with the only two regions that legitimately differ
between two links of the same source masked out:

  * the LC_UUID payload: Apple's linker (ld-1267 and later) assigns a random
    UUID on every link, and -reproducible, SOURCE_DATE_EPOCH and ZERO_AR_DATE
    do not change that; -no_uuid removes it but dyld then refuses to load
    the binary ("missing LC_UUID load command");
  * the LC_CODE_SIGNATURE blob: the linker's ad-hoc signature covers the
    UUID, so it differs too, and it is replaced by a real signature at
    bundle time anyway.

Everything else, code, strings, layout, load commands, must match byte for
byte. A one-word change in a string constant makes the two unequal.

Written 9 September 2026 so scripts/build-local-mac.sh can prove the tracked
sidecar launcher (src-tauri/binaries/jura-sidecar-aarch64-apple-darwin) is
exactly its source (src-tauri/sidecar-launcher/jura-sidecar-launcher.c)
compiled, on every build, without the build having to overwrite a tracked
file. Standard library only.
"""

import struct
import sys

MH_MAGIC_64 = 0xFEEDFACF
LC_UUID = 0x1B
LC_CODE_SIGNATURE = 0x1D


def normalise(path: str) -> bytes:
    b = bytearray(open(path, "rb").read())
    if len(b) < 32:
        raise ValueError(f"{path}: too short to be a Mach-O")
    magic, _cpu, _sub, _ftype, ncmds, _sizeofcmds, _flags, _reserved = struct.unpack_from(
        "<IIIIIIII", b, 0
    )
    if magic != MH_MAGIC_64:
        raise ValueError(f"{path}: not a little-endian 64-bit Mach-O (magic {magic:#x})")
    off = 32
    for _ in range(ncmds):
        cmd, cmdsize = struct.unpack_from("<II", b, off)
        if cmdsize < 8:
            raise ValueError(f"{path}: malformed load command at {off}")
        if cmd == LC_UUID:
            b[off + 8 : off + 24] = b"\0" * 16
        elif cmd == LC_CODE_SIGNATURE:
            dataoff, datasize = struct.unpack_from("<II", b, off + 8)
            b[dataoff : dataoff + datasize] = b"\0" * datasize
        off += cmdsize
    return bytes(b)


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__.strip().splitlines()[0], file=sys.stderr)
        print("usage: macho-equal.py A B", file=sys.stderr)
        return 2
    try:
        a, b = normalise(argv[1]), normalise(argv[2])
    except (OSError, ValueError) as e:
        print(f"macho-equal: {e}", file=sys.stderr)
        return 2
    if a == b:
        print(f"macho-equal: identical code ({len(a)} bytes, UUID and signature masked)")
        return 0
    print("macho-equal: DIFFERENT", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
