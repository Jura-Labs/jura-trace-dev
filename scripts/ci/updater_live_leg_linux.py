#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""The Linux half of the updater live leg (v1.2.0 plan, item A11).

Launches a publicly downloaded AppImage of the PREVIOUS release, presses
Check for Updates in its Settings page, and reports what the app made of the
answer the real update service gave it. Run by
.github/workflows/updater-live-leg-linux.yml, which explains why this is a
separate file from the Windows leg and what a green run does and does not
prove.

How the app is driven. Tauri on Linux is WebKitGTK, which has no Chrome
DevTools port, so the Windows route does not exist here. WebKitWebDriver does:
it launches the binary it is given and speaks W3C WebDriver to it, provided
the web view allows automation. tauri-runtime-wry allows it when
TAURI_WEBVIEW_AUTOMATION=true is in the environment (lib.rs:4792 in 2.11.4,
the version v1.1.0 shipped), so the unmodified shipped AppImage can be driven.
tauri-driver is a proxy that sets that variable and renames one capability.
Both are done here directly, which saves a cargo install on the runner.

What is asserted is durable: the version the About section reports before the
press and after a FRESH launch of the same file, and the SHA-256 of the
AppImage on disk before and after. The sentences the Settings page shows on
the way are recorded when they are caught and nothing depends on catching
them, for the reason run 34570877379 of the Windows leg taught.

Every helper returns a record saying how far it got, never a bare None
(BL-SILENT-001). A run that could not look at the app must not read like a
run that looked and found nothing.

Usage:
    updater_live_leg_linux.py --appimage PATH --previous-tag v1.1.0 \
        --expected 1.2.0 [--rehearsal] [--published-sha256 HEX]

Rehearsal mode expects NO update: the app must say it is up to date, stay on
the version it started on, and leave its own file untouched. It exists
because v1.1.0 is the first release to carry an AppImage, so until v1.2.0 is
published there is nothing for a Linux client to update to, and the driving
half of this leg would otherwise be first exercised on release day.
"""

import argparse
import hashlib
import ipaddress
import json
import os
import re
import signal
import socket
import ssl
import subprocess
import sys
import time
import urllib.error
import urllib.request

MANIFEST_URL = "https://juralabs.org/api/updates/latest.json"
MANIFEST_HOST = "juralabs.org"
PLATFORM_KEY = "linux-x86_64"
DRIVER_PORT = 4444
DRIVER = f"http://127.0.0.1:{DRIVER_PORT}"

# The copy v1.1.0 ships, from `git show v1.1.0:ui/src/routes/settings/+page.svelte`
# and `v1.1.0:ui/src/lib/updater.ts`. The app under test is the previous tag's
# build, so rewording any of these on main does not reach this test until the
# release after next.
UP_TO_DATE = "Jura Trace is up to date."
RELAUNCH_FAILED = "The update is installed. Quit Jura Trace and open it again to finish."
UNREACHABLE = "Could not reach the update service"

# The probe, as the body of a function WebDriver calls with (EXPECTED, PRESS).
# The values arrive as arguments and are never spliced into the text, which is
# the fault that cost the Windows leg four runs.
#
# textContent rather than innerText, so the page is read even behind the
# first-run onboarding overlay. The button is pressed at most once per page,
# because a second press would restart the download.
PROBE_JS = r"""
var EXPECTED = arguments[0];
var PRESS = arguments[1];
try {
  var text = (document.body ? document.body.textContent : "").replace(/\s+/g, " ").trim();
  if (location.href.indexOf("://localhost") === -1 && location.href.indexOf("tauri.localhost") === -1) {
    return JSON.stringify({ stage: "not-the-app", url: location.href, ready: document.readyState });
  }
  if (location.pathname.indexOf("settings") === -1) {
    location.href = "/settings";
    return JSON.stringify({ stage: "navigating", url: location.href, ready: document.readyState });
  }
  var btns = Array.prototype.slice.call(document.querySelectorAll("button"));
  var b = null;
  for (var i = 0; i < btns.length; i++) {
    var label = btns[i].textContent || "";
    if (/Check for Updates|Checking|Downloading update|Installing, restarting/i.test(label)) { b = btns[i]; break; }
  }
  var vm = text.match(/About\s+Version\s+([0-9][0-9A-Za-z.\-+]*)/);
  var appVersion = vm ? vm[1] : null;
  var at = text.indexOf("About Version");
  var about = at === -1 ? "" : text.slice(at, at + 900);
  var out = {
    stage: "waiting-for-button",
    appVersion: appVersion,
    hasButton: !!b,
    buttonLabel: b ? (b.textContent || "").replace(/\s+/g, " ").trim() : null,
    buttonDisabled: b ? !!b.disabled : null,
    url: location.href,
    ready: document.readyState,
    textLength: text.length,
    about: about
  };
  if (!PRESS) { out.stage = appVersion ? "read" : "waiting-for-version"; return JSON.stringify(out); }
  if (b && appVersion && !window.__juraClicked && /Check for Updates/i.test(out.buttonLabel)) {
    window.__juraClicked = true;
    b.click();
    out.stage = "clicked";
    return JSON.stringify(out);
  }
  if (window.__juraClicked) { out.stage = "watching"; }
  return JSON.stringify(out);
} catch (e) {
  return JSON.stringify({ stage: "error", message: String(e) });
}
"""


def say(msg=""):
    print(msg, flush=True)


def error(msg):
    say(f"::error::{msg}")


def warning(msg):
    say(f"::warning::{msg}")


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


# ── the claim that it is live is asserted, not assumed ──────────────────────


def assert_live(expected, rehearsal):
    """Refuse to continue unless this machine is talking to the real service.

    Returns the manifest. The four checks mirror the Windows leg's, and the
    fourth is the one that matters: fetch the manifest as the app will and
    look at the certificate the server presented and the document it returned.
    """
    fail = False

    with open("/etc/hosts", encoding="utf-8", errors="replace") as f:
        if "juralabs" in f.read():
            error("/etc/hosts mentions juralabs. This runner is not clean.")
            fail = True
        else:
            say("OK: /etc/hosts does not mention juralabs.")

    addrs = sorted({info[4][0] for info in socket.getaddrinfo(MANIFEST_HOST, 443)})
    say(f"{MANIFEST_HOST} resolves to: {', '.join(addrs)}")
    for a in addrs:
        if not ipaddress.ip_address(a).is_global:
            error(f"{MANIFEST_HOST} resolves to a non-public address ({a}).")
            fail = True
    if not fail:
        say("OK: every address is public.")

    # Only a certificate of OURS is suspicious. The staging leg mints one
    # with jura in its name, and that is the only thing specific to us.
    ours = []
    for d in ("/usr/local/share/ca-certificates", "/etc/ssl/certs"):
        if os.path.isdir(d):
            ours += [os.path.join(d, n) for n in os.listdir(d) if "jura" in n.lower()]
    if ours:
        error("A certificate mentioning jura is installed on this machine: " + ", ".join(ours))
        fail = True
    else:
        say("OK: no jura certificate in the system trust store.")

    ctx = ssl.create_default_context()
    with socket.create_connection((MANIFEST_HOST, 443), timeout=20) as raw:
        with ctx.wrap_socket(raw, server_hostname=MANIFEST_HOST) as tls:
            cert = tls.getpeercert()
    issuer = ", ".join(f"{k}={v}" for rdn in cert.get("issuer", ()) for k, v in rdn)
    say(f"server certificate issuer : {issuer}")
    if re.search(r"jura|throwaway", issuer, re.I):
        error(f"The certificate presented for {MANIFEST_HOST} was issued by us.")
        fail = True
    else:
        say("OK: the certificate was issued by a public authority, not by this test.")

    req = urllib.request.Request(MANIFEST_URL, headers={"User-Agent": "jura-trace-live-leg-precheck"})
    with urllib.request.urlopen(req, timeout=30) as r:
        manifest = json.loads(r.read())
    platforms = sorted(manifest.get("platforms", {}))
    say(f"live manifest version   : {manifest.get('version')}")
    say(f"live manifest platforms : {', '.join(platforms)}")
    entry = manifest.get("platforms", {}).get(PLATFORM_KEY)
    if not entry:
        error(f"The live manifest has no {PLATFORM_KEY} entry, so there is nothing for this client to be offered.")
        fail = True
    else:
        say(f"OK: the live manifest carries a {PLATFORM_KEY} entry: {entry.get('url')}")
        # base64 of "untrusted comment:", which is how a minisign signature begins.
        if not str(entry.get("signature", "")).startswith("dW50cnVzdGVk"):
            error(f"The {PLATFORM_KEY} signature field does not contain a minisign signature (BL-REL-002).")
            fail = True
        else:
            say(f"OK: the {PLATFORM_KEY} signature is a minisign signature, not a URL.")
        if not str(entry.get("url", "")).endswith(".AppImage"):
            error(f"The {PLATFORM_KEY} entry does not point at an AppImage, and an AppImage client installs what it is sent.")
            fail = True

    # Asked before the 850 MB download rather than after it. If the service
    # is not offering what this run was dispatched to prove, the run cannot
    # prove it, and saying so here costs seconds.
    if str(manifest.get("version")) != expected:
        what = "the version the installed app is already on" if rehearsal else "the version this run expects to be installed"
        error(f"The live manifest offers {manifest.get('version')}, and {what} is {expected}.")
        say("This is a failure of the TEST's inputs, not evidence about the client.")
        fail = True

    if fail:
        say("Refusing to continue. A live leg that is not live proves nothing.")
        sys.exit(1)
    return manifest


# ── WebDriver, spoken directly ──────────────────────────────────────────────


def wd(method, path, body=None, timeout=30):
    """One WebDriver call. Always returns a record of how far it got."""
    rec = {"ok": False, "status": None, "value": None, "failure": None}
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(
        DRIVER + path, data=data, method=method, headers={"Content-Type": "application/json"}
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            rec["status"] = r.status
            rec["value"] = json.loads(r.read()).get("value")
            rec["ok"] = True
    except urllib.error.HTTPError as e:
        rec["status"] = e.code
        try:
            v = json.loads(e.read()).get("value") or {}
            rec["failure"] = f"{v.get('error')}: {v.get('message')}"
        except Exception:  # noqa: BLE001, the body is diagnostic only
            rec["failure"] = f"HTTP {e.code} with an unreadable body"
    except Exception as e:  # noqa: BLE001, every failure becomes a record
        rec["failure"] = f"{type(e).__name__}: {e}"
    return rec


def start_driver(log_path):
    env = dict(os.environ, TAURI_WEBVIEW_AUTOMATION="true")
    log = open(log_path, "ab")
    proc = subprocess.Popen(
        ["WebKitWebDriver", f"--port={DRIVER_PORT}", "--host=127.0.0.1"],
        stdout=log, stderr=subprocess.STDOUT, env=env,
    )
    deadline = time.time() + 30
    last = None
    while time.time() < deadline:
        if proc.poll() is not None:
            return proc, f"WebKitWebDriver exited at once with code {proc.returncode}"
        last = wd("GET", "/status", timeout=3)
        if last["ok"]:
            return proc, None
        time.sleep(0.5)
    return proc, f"WebKitWebDriver never answered /status. Last attempt: {last and last['failure']}"


def open_session(appimage):
    """Have the driver launch the AppImage. Returns (session id or None, record)."""
    caps = {"capabilities": {"alwaysMatch": {"webkitgtk:browserOptions": {"binary": appimage, "args": []}}}}
    rec = wd("POST", "/session", caps, timeout=240)
    sid = None
    if rec["ok"] and isinstance(rec["value"], dict):
        sid = rec["value"].get("sessionId")
        if not sid:
            rec["failure"] = f"the driver answered without a sessionId: {json.dumps(rec['value'])[:400]}"
    return sid, rec


def probe(sid, expected, press):
    """Evaluate the probe. Returns (parsed object or None, why not)."""
    rec = wd("POST", f"/session/{sid}/execute/sync", {"script": PROBE_JS, "args": [expected, press]}, timeout=20)
    if not rec["ok"]:
        return None, f"no reply. {rec['failure']}"
    if not isinstance(rec["value"], str):
        return None, f"the result was not the string the probe promised: {json.dumps(rec['value'])[:300]}"
    try:
        return json.loads(rec["value"]), None
    except ValueError:
        return None, f"the result was not JSON: {rec['value'][:300]}"


# ── the app's processes ─────────────────────────────────────────────────────


def app_processes(appimage):
    """(pid, argv0) for every process that is the AppImage or runs from its mount.

    Matched on argv[0] and not on the whole command line, because this script
    and its wrappers carry the AppImage path as an argument.
    """
    found = []
    for name in os.listdir("/proc"):
        if not name.isdigit():
            continue
        try:
            with open(f"/proc/{name}/cmdline", "rb") as f:
                argv0 = f.read().split(b"\0")[0].decode(errors="replace")
        except OSError:
            continue
        if argv0 == appimage or argv0.startswith("/tmp/.mount_"):
            found.append((int(name), argv0))
    return found


def stop_app(appimage):
    for sig in (signal.SIGTERM, signal.SIGKILL):
        procs = app_processes(appimage)
        if not procs:
            return
        for pid, _ in procs:
            try:
                os.kill(pid, sig)
            except OSError:
                pass
        time.sleep(4)


# ── the two observations ────────────────────────────────────────────────────


def press_and_watch(appimage, expected, window_s, record):
    """Launch, press Check for Updates once, and watch until the app settles."""
    sid, rec = open_session(appimage)
    record["first_session"] = {"opened": bool(sid), "failure": rec["failure"]}
    if not sid:
        return
    say(f"  session open, the driver launched {os.path.basename(appimage)}")

    started = time.time()
    deadline = started + window_s
    trail = record["probe_outcomes"]
    last_line = ""
    clicked_at = None
    seen_busy = False
    idle_after_click = 0
    lost = 0
    pattern = re.compile(r"Version " + re.escape(expected) + r" is available")

    while time.time() < deadline:
        time.sleep(3)
        at = int(time.time() - started)
        p, why = probe(sid, expected, True)
        if p is None:
            line = f"nothing usable. {why}"
        else:
            line = (
                f"stage={p.get('stage')} url={p.get('url')} ready={p.get('ready')} version={p.get('appVersion')} "
                f"button={p.get('buttonLabel')!r} disabled={p.get('buttonDisabled')} textLength={p.get('textLength')}"
            )
        # One line per distinct outcome, so a run of identical probes shows
        # as a gap between two timestamps and not as two hundred lines.
        if line != last_line:
            last_line = line
            say(f"  probe at {at}s: {line}")
            trail.append(f"{at}s {line}")

        if p is None:
            # After the press, losing the session is the app exiting, which
            # is what a relaunch looks like from here. Before it, it is the
            # test failing to hold on to the app.
            lost += 1
            if lost >= 3:
                record["session_lost_at_s"] = at
                record["session_lost_after_press"] = clicked_at is not None
                say(f"  the session was lost at {at}s, {'after' if clicked_at is not None else 'BEFORE'} the press")
                return
            continue
        lost = 0
        record["last_probe_of_any_kind"] = p
        if p.get("textLength"):
            record["dom_read"] = True
        if p.get("stage") == "error":
            record["page_threw"] = p.get("message")
            continue

        if p.get("appVersion") and clicked_at is None and not record["version_before"]:
            record["version_before"] = p["appVersion"]
            say(f"  the launched app calls itself {p['appVersion']}")
        if p.get("stage") == "clicked":
            clicked_at = time.time()
            record["button_pressed"] = True
            say(f"  pressed Check for Updates on {p.get('url')}")
            continue
        if clicked_at is None:
            continue

        about = p.get("about") or ""
        record["about_section_last"] = about
        m = pattern.search(about)
        if m:
            record["offer_sentence"] = m.group(0)
        if p.get("buttonDisabled"):
            seen_busy = True
            if p.get("buttonLabel") and p["buttonLabel"] not in record["button_labels_seen"]:
                record["button_labels_seen"].append(p["buttonLabel"])
        if UP_TO_DATE in about:
            record["said_up_to_date"] = True
            say(f"  the app says: {UP_TO_DATE}")
            break
        if RELAUNCH_FAILED in about:
            record["said_relaunch_failed"] = True
            say(f"  the app says: {RELAUNCH_FAILED}")
            break
        # The button is enabled again and none of the sentences above is on
        # the page: the state machine ended in `error`. Two probes in a row,
        # and only once the button has been seen busy or thirty seconds have
        # passed, so the gap between the press and `checking` is not read as
        # an ending.
        if p.get("buttonDisabled") is False and (seen_busy or time.time() - clicked_at > 30):
            idle_after_click += 1
            if idle_after_click >= 2:
                record["ended_in_error_state"] = True
                say("  the button is enabled again and the app reports neither an update nor up to date")
                say(f"  what the About section shows: {about}")
                break
        else:
            idle_after_click = 0
    else:
        record["window_expired"] = True

    wd("DELETE", f"/session/{sid}", timeout=20)


def read_version_fresh(appimage, expected, record):
    """Launch the file that is on disk now and read what it calls itself."""
    sid, rec = open_session(appimage)
    record["second_session"] = {"opened": bool(sid), "failure": rec["failure"]}
    if not sid:
        return
    started = time.time()
    last_line = ""
    while time.time() - started < 240:
        time.sleep(3)
        p, why = probe(sid, expected, False)
        line = f"nothing usable. {why}" if p is None else f"stage={p.get('stage')} url={p.get('url')} version={p.get('appVersion')}"
        if line != last_line:
            last_line = line
            at = int(time.time() - started)
            say(f"  fresh launch, probe at {at}s: {line}")
            record["fresh_launch_outcomes"].append(f"{at}s {line}")
        if p and p.get("appVersion"):
            record["version_after"] = p["appVersion"]
            break
    wd("DELETE", f"/session/{sid}", timeout=20)


# ── the verdict ─────────────────────────────────────────────────────────────


def verdict(record, args):
    """Return (exit code, one-line verdict). Prints the reasoning."""
    prev, expected = args.previous_tag, args.expected
    r = record

    # Failures of the TEST first, each with its own message.
    if r["driver_failure"]:
        error(f"WebKitWebDriver could not be started, so the app was never launched. {r['driver_failure']}")
        say("This is a failure of the TEST, not evidence about the client.")
        return 1, None
    if not r["first_session"].get("opened"):
        error("The driver could not launch the AppImage and open a session, so nothing was observed.")
        say("This is a failure of the TEST, not evidence about the client.")
        say(f"What the driver said: {r['first_session'].get('failure')}")
        say("driver.log is in the artefact. An AppImage that will not start at all (no FUSE, a")
        say("missing library) and a web view that refused automation both end here.")
        return 1, None
    if not r["dom_read"]:
        error("A session opened but no page returned a document, so nothing was observed.")
        say("This is a failure of the TEST, not evidence about the client.")
        if r["page_threw"]:
            say(f"The probe threw inside the page: {r['page_threw']}")
        say(f"The last probe of any kind: {json.dumps(r['last_probe_of_any_kind'])[:1500]}")
        return 1, None
    if not r["button_pressed"]:
        error("Check for Updates was never pressed, so the client was never asked.")
        say(f"This is a failure of the TEST, not evidence about the client. Either the Settings")
        say(f"route did not load, the About section carried no version, or no button carries that")
        say(f"label in {prev}.")
        say(f"The last probe of any kind: {json.dumps(r['last_probe_of_any_kind'])[:1500]}")
        return 1, None
    say(f"OK: Check for Updates was pressed on a launched {prev}.")
    if not r["second_session"].get("opened") or not r["version_after"]:
        error("The file on disk afterwards could not be launched and read, so what was installed is unknown.")
        say(f"What the driver said: {r['second_session'].get('failure')}")
        say(f"sha256 before the press: {r['sha256_before']}")
        say(f"sha256 after it        : {r['sha256_after']}")
        say("If the two differ, the update replaced the file and the replacement does not start,")
        say("which would be a finding about the release and not about this test.")
        return 1, None

    before, after = r["version_before"], r["version_after"]
    changed = r["sha256_before"] != r["sha256_after"]

    if args.rehearsal:
        ok = True
        if before != expected:
            error(f"Rehearsal expects the launched app to be on {expected} already, and it calls itself {before}.")
            ok = False
        if not r["said_up_to_date"]:
            error("The app was asked and did not say it is up to date.")
            say(f"What the About section showed last: {r['about_section_last']}")
            say("The manifest offers the version the app is on, so anything other than up to date")
            say("means the check failed or the client misread the answer.")
            ok = False
        if changed:
            error("The AppImage on disk changed although there was nothing to install.")
            ok = False
        if after != expected:
            error(f"A fresh launch afterwards calls itself {after}, not {expected}.")
            ok = False
        if not ok:
            return 1, None
        say(f"OK: REHEARSAL. A publicly downloaded {prev} AppImage, talking to the live service,")
        say(f"    was driven to Settings, asked, and said it is up to date. It stayed on {after}")
        say("    and its file is byte for byte what was downloaded.")
        say("    This proves the driving and the check. It does NOT prove an install: nothing")
        say("    was offered, so nothing was downloaded, verified or written.")
        return 0, f"REHEARSAL: {before} asked, up to date, unchanged"

    if before == expected:
        error(f"The launched {prev} already called itself {expected} before anything was pressed.")
        say("This is a failure of the TEST: the wrong AppImage was fetched.")
        return 1, None
    if after != expected:
        error(f"A pristine {prev} AppImage was asked, and did not end up on {expected}.")
        say(f"It called itself {before} before the press and {after} on a fresh launch afterwards.")
        say("The button was pressed, so this is the client's answer rather than a gap in the test.")
        if r["offer_sentence"]:
            say(f'The app did say: "{r["offer_sentence"]}", so it was offered and did not arrive.')
        if r["said_up_to_date"]:
            say("The app said it is up to date, so it did not accept the published version as newer.")
        say(f"The file on disk {'changed' if changed else 'did NOT change'}.")
        say(f"What the About section showed last: {r['about_section_last']}")
        return 1, None
    if not changed:
        error(f"The app now calls itself {expected} and the file on disk is unchanged, which cannot both be true of an update.")
        return 1, None
    if args.published_sha256 and r["sha256_after"] != args.published_sha256.lower():
        error("The AppImage on disk after the update is not the published one.")
        say(f"on disk  : {r['sha256_after']}")
        say(f"published: {args.published_sha256.lower()}")
        return 1, None

    say(f"OK: a pristine {prev} AppImage, talking to the live service over public DNS and the")
    say(f"    real certificate chain, fetched and installed {expected}.")
    say(f"    It called itself {before} before the press and {after} on a fresh launch afterwards.")
    if args.published_sha256:
        say("    The file on disk is byte for byte the published AppImage.")
    else:
        warning("No published SHA-256 was supplied, so the installed file was not compared with the release asset.")
    if r["offer_sentence"]:
        say(f'    The sentence caught in flight: "{r["offer_sentence"]}"')

    # Reported, not asserted. Whether an AppImage relaunches itself after
    # replacing its own file is a first observation in v1.2.0 and the plan
    # says so. It is loud either way.
    if r["said_relaunch_failed"]:
        warning("The update installed, and the app could NOT restart itself: it asked the user to quit and reopen.")
    elif r["relaunched_by_itself"]:
        say("    The app restarted itself after installing.")
    else:
        warning("The update installed, but no relaunched app process was seen afterwards. The install is proved; the unattended restart is not.")
    return 0, f"{before} -> {after}"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--appimage", required=True, help="path to the previous version's AppImage, already executable")
    ap.add_argument("--previous-tag", required=True)
    ap.add_argument("--expected", required=True, help="version the app must be on afterwards")
    ap.add_argument("--rehearsal", action="store_true", help="expect up to date and no change")
    ap.add_argument("--published-sha256", default="", help="SHA-256 of the published AppImage for --expected")
    ap.add_argument("--window", type=int, default=900, help="seconds to watch after launch")
    ap.add_argument("--out", default=".", help="directory for verdict.log, observed-verdict.txt and driver.log")
    args = ap.parse_args()
    appimage = os.path.abspath(args.appimage)

    assert_live(args.expected, args.rehearsal)

    record = {
        "previous_tag": args.previous_tag,
        "expected": args.expected,
        "rehearsal": args.rehearsal,
        "driver_failure": None,
        "first_session": {},
        "dom_read": False,
        "page_threw": None,
        "button_pressed": False,
        "version_before": None,
        "version_after": None,
        "offer_sentence": None,
        "button_labels_seen": [],
        "said_up_to_date": False,
        "said_relaunch_failed": False,
        "ended_in_error_state": False,
        "window_expired": False,
        "session_lost_at_s": None,
        "session_lost_after_press": None,
        "relaunched_by_itself": None,
        "about_section_last": None,
        "sha256_before": sha256_of(appimage),
        "sha256_after": None,
        "published_sha256": args.published_sha256.lower() or None,
        "second_session": {},
        "last_probe_of_any_kind": None,
        "probe_outcomes": [],
        "fresh_launch_outcomes": [],
    }
    say(f"sha256 of the downloaded AppImage: {record['sha256_before']}")

    driver, failure = start_driver(os.path.join(args.out, "driver.log"))
    record["driver_failure"] = failure
    try:
        if not failure:
            say("── launch, press, watch ──")
            press_and_watch(appimage, args.expected, args.window, record)

            # A relaunch is the app starting itself again, so it shows as an
            # app process that outlives the session this test opened.
            if record["session_lost_after_press"]:
                seen = []
                for _ in range(15):
                    time.sleep(3)
                    seen = app_processes(appimage)
                    if seen:
                        break
                record["relaunched_by_itself"] = bool(seen)
                say(f"  app processes after the session was lost: {seen if seen else 'none'}")

            stop_app(appimage)
            record["sha256_after"] = sha256_of(appimage) if os.path.exists(appimage) else None
            say(f"sha256 of the AppImage on disk now: {record['sha256_after']}")

            if record["button_pressed"] and record["sha256_after"]:
                say("── fresh launch of what is on disk now ──")
                read_version_fresh(appimage, args.expected, record)
    finally:
        stop_app(appimage)
        driver.terminate()
        with open(os.path.join(args.out, "verdict.log"), "w", encoding="utf-8") as f:
            json.dump(record, f, indent=2)

    code, line = verdict(record, args)
    if line:
        with open(os.path.join(args.out, "observed-verdict.txt"), "w", encoding="utf-8") as f:
            f.write(line + "\n")
    return code


if __name__ == "__main__":
    sys.exit(main())
