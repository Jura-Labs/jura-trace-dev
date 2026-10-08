/* SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * jura-sidecar-launcher-win: the Windows launcher for the bundled PyInstaller
 * --onedir sidecar (v1.2.0 B1). The macOS counterpart is
 * jura-sidecar-launcher.c; the job is the same, the mechanics are not.
 *
 * Tauri's externalBin places this at <install root>\jura-sidecar.exe. The
 * real PyInstaller bootloader and its _internal\ tree ship as Tauri
 * resources, which on Windows land directly under the install root, so the
 * bootloader is at <install root>\sidecar-bundle\jura-sidecar.exe. That
 * layout was observed, not inferred: the B1 probe (run 35699529154, 22
 * September 2026) installed both the MSI and the NSIS build and found all
 * 1,426 files there. There is no resources\ level.
 *
 * Why this cannot be the macOS launcher with an #ifdef:
 *
 * 1. Windows has no execv. The CRT's _execv starts a new process and exits
 *    the caller, so Tauri would hold a handle to a launcher that has already
 *    gone while the sidecar ran on unparented. This launcher stays alive,
 *    waits for the bootloader, and exits with its exit code.
 *
 * 2. Staying alive means two processes, and the rc.21 smoke test (23 April
 *    2026) showed what an orphaned sidecar costs on Windows: it holds its own
 *    files locked and the next install fails. So the two are tied together
 *    both ways. The bootloader runs inside a Job Object created with
 *    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE; the launcher holds the only handle
 *    to that job, so when the launcher dies for any reason, including
 *    TerminateProcess from Tauri's CommandChild::kill, the kernel closes the
 *    handle and kills everything in the job. In the other direction, the
 *    launcher is blocked in WaitForSingleObject on the bootloader and exits
 *    as soon as it does.
 *
 * 3. Tauri reads the sidecar's stdout and stderr and logs every line
 *    (startup.rs, spawn_sidecar). Those are the only startup diagnostics the
 *    product has, so the bootloader is given the launcher's own standard
 *    handles explicitly rather than trusting defaults.
 *
 * It is C, not a .cmd, because externalBin resolves to an .exe on Windows and
 * because a batch file cannot carry an Authenticode signature. The release
 * workflow compiles it on the Windows runner; see src-tauri/binaries/README.md.
 */
#define WIN32_LEAN_AND_MEAN
#define UNICODE
#define _UNICODE
#include <windows.h>
#include <stdio.h>
#include <wchar.h>

static const wchar_t kRelBootloader[] = L"sidecar-bundle\\jura-sidecar.exe";

/* Longest path Windows supports with the \\?\ prefix, in wide characters. */
#define LONGEST_PATH 32768

static int fail(const wchar_t *what) {
    fwprintf(stderr, L"B1: %ls (Win32 error %lu)\n", what, GetLastError());
    return 127;
}

/* The command line after argv[0], untouched. Parsing argv and quoting it
 * back is where launchers corrupt arguments, so this finds where argv[0]
 * ends by the rule CommandLineToArgvW uses for it (quoted: up to the next
 * quote, no escapes; unquoted: up to the first space or tab) and passes the
 * remainder through byte for byte. */
static const wchar_t *args_after_argv0(const wchar_t *cmd) {
    const wchar_t *p = cmd;
    if (*p == L'"') {
        p++;
        while (*p && *p != L'"') p++;
        if (*p == L'"') p++;
    } else {
        while (*p && *p != L' ' && *p != L'\t') p++;
    }
    while (*p == L' ' || *p == L'\t') p++;
    return p;
}

static void make_inheritable(DWORD which, STARTUPINFOW *si, HANDLE *slot) {
    HANDLE h = GetStdHandle(which);
    if (h != NULL && h != INVALID_HANDLE_VALUE) {
        SetHandleInformation(h, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT);
    }
    *slot = h;
    si->dwFlags |= STARTF_USESTDHANDLES;
}

int wmain(void) {
    static wchar_t self[LONGEST_PATH];
    DWORD n = GetModuleFileNameW(NULL, self, LONGEST_PATH);
    if (n == 0 || n >= LONGEST_PATH) {
        return fail(L"cannot determine launcher path");
    }

    wchar_t *slash = wcsrchr(self, L'\\');
    if (slash == NULL) {
        fwprintf(stderr, L"B1: launcher path has no directory component: %ls\n", self);
        return 127;
    }
    slash[1] = L'\0';

    static wchar_t bootloader[LONGEST_PATH];
    if (_snwprintf_s(bootloader, LONGEST_PATH, _TRUNCATE, L"%ls%ls", self, kRelBootloader) < 0) {
        fwprintf(stderr, L"B1: bootloader path too long\n");
        return 127;
    }

    DWORD attrs = GetFileAttributesW(bootloader);
    if (attrs == INVALID_FILE_ATTRIBUTES || (attrs & FILE_ATTRIBUTE_DIRECTORY)) {
        fwprintf(stderr, L"B1: sidecar bootloader not found at %ls\n", bootloader);
        fwprintf(stderr, L"B1: the installer was built without the sidecar-bundle resource;\n");
        fwprintf(stderr, L"B1: the release workflow stages it from the PyInstaller --onedir output.\n");
        return 127;
    }

    /* CreateProcessW may write to the command line buffer, so it must be a
     * mutable copy. */
    const wchar_t *rest = args_after_argv0(GetCommandLineW());
    size_t cmd_len = wcslen(bootloader) + wcslen(rest) + 4;
    wchar_t *cmd = HeapAlloc(GetProcessHeap(), 0, cmd_len * sizeof(wchar_t));
    if (cmd == NULL) {
        return fail(L"out of memory building the command line");
    }
    _snwprintf_s(cmd, cmd_len, _TRUNCATE, *rest ? L"\"%ls\" %ls" : L"\"%ls\"%ls", bootloader, rest);

    /* Not inheritable (NULL security attributes). If the bootloader inherited
     * this handle it would keep the job open after the launcher died, and
     * KILL_ON_JOB_CLOSE would never fire. */
    HANDLE job = CreateJobObjectW(NULL, NULL);
    if (job == NULL) {
        return fail(L"CreateJobObject failed");
    }
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits;
    ZeroMemory(&limits, sizeof(limits));
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    if (!SetInformationJobObject(job, JobObjectExtendedLimitInformation, &limits, sizeof(limits))) {
        return fail(L"SetInformationJobObject failed");
    }

    STARTUPINFOW si;
    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    make_inheritable(STD_INPUT_HANDLE, &si, &si.hStdInput);
    make_inheritable(STD_OUTPUT_HANDLE, &si, &si.hStdOutput);
    make_inheritable(STD_ERROR_HANDLE, &si, &si.hStdError);

    /* Suspended, so the bootloader cannot run a single instruction outside
     * the job. It shares this process's (hidden) console, so no window. */
    PROCESS_INFORMATION pi;
    ZeroMemory(&pi, sizeof(pi));
    if (!CreateProcessW(bootloader, cmd, NULL, NULL, TRUE, CREATE_SUSPENDED, NULL, NULL, &si, &pi)) {
        return fail(L"CreateProcess of the sidecar bootloader failed");
    }
    HeapFree(GetProcessHeap(), 0, cmd);

    if (!AssignProcessToJobObject(job, pi.hProcess)) {
        /* Refuse rather than run untied: an orphan here is the rc.21 locked
         * file fault. Nested jobs work on every Windows Tauri 2 supports, so
         * this should never happen, and if it does it should be loud. */
        int rc = fail(L"AssignProcessToJobObject failed; refusing to run the sidecar untied");
        TerminateProcess(pi.hProcess, 127);
        return rc;
    }

    if (ResumeThread(pi.hThread) == (DWORD)-1) {
        int rc = fail(L"ResumeThread failed");
        TerminateProcess(pi.hProcess, 127);
        return rc;
    }
    CloseHandle(pi.hThread);

    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD code = 127;
    if (!GetExitCodeProcess(pi.hProcess, &code)) {
        return fail(L"GetExitCodeProcess failed");
    }
    CloseHandle(pi.hProcess);
    /* Returning closes the job handle, which would kill anything the
     * bootloader left behind in it. That is intended. */
    return (int)code;
}
