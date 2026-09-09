#!/usr/bin/env python3
"""
macho-equal.py — are two 64-bit Mach-O files the same code?

    python3 scripts/macho-equal.py A B      exit 0 if equal, 1 if not, 2 on error

Two links of the same source differ in exactly two places, and this tool
removes both before comparing:

  * the code signature. The linker ad-hoc signs every arm64 output, the
    signature's CodeDirectory embeds the output file's basename, and its size
    changes the __LINKEDIT segment and the LC_CODE_SIGNATURE load command
    with it, so masking the blob in place is not enough. Each file is copied
    to a temp dir and `codesign --remove-signature` rewrites the copy without
    the signature, adjusting the load commands consistently. Tauri replaces
    the ad-hoc signature with the Developer ID one at bundle time anyway.
  * the LC_UUID payload. Apple's linker (ld-1267) assigns a random UUID on
    every link; -reproducible, SOURCE_DATE_EPOCH and ZERO_AR_DATE do not
    change that, and -no_uuid yields a binary dyld refuses to load. The
    16 bytes are zeroed in both copies.

Everything else, code, strings, layout, load commands, must then match byte
for byte. A one-word change in a string constant makes the two unequal.

Written 9 September 2026 so scripts/build-local-mac.sh can prove the tracked
sidecar launcher (src-tauri/binaries/jura-sidecar-aarch64-apple-darwin) is
exactly its source (src-tauri/sidecar-launcher/jura-sidecar-launcher.c)
compiled, on every build, without overwriting a tracked file. Standard
library plus the system codesign tool.
"""

import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

MH_MAGIC_64 = 0xFEEDFACF
LC_UUID = 0x1B


def stripped_copy(src: str, tmp: Path, name: str) -> Path:
    dst = tmp / name
    shutil.copyfile(src, dst)
    dst.chmod(0o755)
    r = subprocess.run(
        ["codesign", "--remove-signature", str(dst)], capture_output=True, text=True
    )
    if r.returncode != 0:
        raise ValueError(
            f"codesign --remove-signature failed on {src}: {r.stderr.strip()}"
        )
    return dst


def normalise(path: Path) -> bytes:
    b = bytearray(path.read_bytes())
    if len(b) < 32:
        raise ValueError(f"{path}: too short to be a Mach-O")
    magic, _cpu, _sub, _ftype, ncmds, _sizeofcmds, _flags, _reserved = (
        struct.unpack_from("<IIIIIIII", b, 0)
    )
    if magic != MH_MAGIC_64:
        raise ValueError(
            f"{path}: not a little-endian 64-bit Mach-O (magic {magic:#x})"
        )
    off = 32
    for _ in range(ncmds):
        cmd, cmdsize = struct.unpack_from("<II", b, off)
        if cmdsize < 8:
            raise ValueError(f"{path}: malformed load command at {off}")
        if cmd == LC_UUID:
            b[off + 8 : off + 24] = b"\0" * 16
        off += cmdsize
    return bytes(b)


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: macho-equal.py A B", file=sys.stderr)
        return 2
    try:
        with tempfile.TemporaryDirectory() as td:
            tmp = Path(td)
            a = normalise(stripped_copy(argv[1], tmp, "a"))
            b = normalise(stripped_copy(argv[2], tmp, "b"))
    except (OSError, ValueError) as e:
        print(f"macho-equal: {e}", file=sys.stderr)
        return 2
    if a == b:
        print(
            f"macho-equal: identical code ({len(a)} bytes, signature removed and UUID masked)"
        )
        return 0
    print("macho-equal: DIFFERENT", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
