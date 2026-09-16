/* SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * jura-sidecar-launcher: the macOS launcher for the bundled PyInstaller
 * --onedir sidecar. Compiled, not a script, and the reason is worth keeping.
 *
 * Tauri's externalBin places this at Contents/MacOS/jura-sidecar. The real
 * PyInstaller bootloader and its _internal/ tree ship as Tauri resources at
 * Contents/Resources/sidecar-bundle/. The bootloader looks for _internal/
 * beside its own executable, so they must be siblings, and Tauri has no
 * option to put _internal/ in Contents/MacOS/. This program exists to exec
 * the bootloader from where it lives (JTV-184 Phase 5).
 *
 * Until 9 September 2026 this was a 38-line shell script. A script's code
 * signature lives in extended attributes, not in the file. The DMG is a
 * filesystem image and carries them, so the DMG passed every check. The
 * updater archive is a tar, Tauri's tar writer drops extended attributes,
 * and the client-side updater extracts with the same library, so the
 * launcher arrived on a user's Mac "not signed at all" and Gatekeeper
 * assessed the updated app as "rejected, no usable signature". A Mach-O
 * binary carries its signature inside the file, which survives the archive.
 * That is the whole reason this is C.
 *
 * Behaviour is identical to the script: resolve the bootloader relative to
 * this executable, refuse loudly if it is missing, otherwise exec it with
 * argv and the environment passed through unchanged.
 */
#include <errno.h>
#include <limits.h>
#include <mach-o/dyld.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static const char *const kRelBootloader = "/../Resources/sidecar-bundle/jura-sidecar";

int main(int argc, char *argv[]) {
    (void)argc; /* argv is passed through whole; the count is not needed */
    char self[PATH_MAX];
    uint32_t size = sizeof(self);
    if (_NSGetExecutablePath(self, &size) != 0) {
        fprintf(stderr, "JTV-184: cannot determine launcher path (buffer %u too small)\n", size);
        return 127;
    }

    char self_real[PATH_MAX];
    if (realpath(self, self_real) == NULL) {
        fprintf(stderr, "JTV-184: realpath(%s) failed: %s\n", self, strerror(errno));
        return 127;
    }

    /* dirname, in place, without the libc dirname's static-buffer caveats. */
    char *slash = strrchr(self_real, '/');
    if (slash == NULL) {
        fprintf(stderr, "JTV-184: launcher path has no directory component: %s\n", self_real);
        return 127;
    }
    *slash = '\0';

    char candidate[PATH_MAX];
    if (snprintf(candidate, sizeof(candidate), "%s%s", self_real, kRelBootloader) >= (int)sizeof(candidate)) {
        fprintf(stderr, "JTV-184: bootloader path too long\n");
        return 127;
    }

    char bootloader[PATH_MAX];
    if (realpath(candidate, bootloader) == NULL || access(bootloader, X_OK) != 0) {
        fprintf(stderr, "JTV-184: sidecar bootloader not found at %s\n", candidate);
        fprintf(stderr, "JTV-184: the .app was built without the sidecar-bundle resource;\n");
        fprintf(stderr, "JTV-184: rebuild with scripts/build-local-mac.sh.\n");
        return 127;
    }

    /* argv[0] becomes the bootloader path; the rest passes through unchanged.
     * PyInstaller locates _internal/ from its own executable path, not argv[0],
     * so this matches what the shell script's `exec "$BOOTLOADER" "$@"` did. */
    argv[0] = bootloader;
    execv(bootloader, argv);

    fprintf(stderr, "JTV-184: exec of %s failed: %s\n", bootloader, strerror(errno));
    return 127;
}
