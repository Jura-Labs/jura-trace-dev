---
title: "Jura Trace v0.9.0-rc.3 — RC Distribution Checklist"
description: "Step-by-step coordinator checklist for distributing the v0.9.0-rc.3 release candidate to pilot testers. Covers pre-distribution checks, per-tester steps, and follow-up schedule."
last-updated: 29 March 2026
status: internal
audience: Pilot coordinator (Juralabs)
---

# Jura Trace v0.9.0-rc.3 — RC Distribution Checklist

This checklist guides the pilot coordinator through distributing the v0.9.0-rc.3 release candidate to the three pilot testers. Work through each section in order. Complete every item before moving to the next section.

---

## Contents

1. [Pre-Distribution Checks](#1-pre-distribution-checks)
2. [Per-Tester Distribution Steps](#2-per-tester-distribution-steps)
3. [Follow-Up Schedule](#3-follow-up-schedule)
4. [Tester Tracking Table](#4-tester-tracking-table)

---

## 1. Pre-Distribution Checks

Complete all items below before contacting any tester. These checks confirm that every asset a tester needs exists and is ready.

### 1.1 Release Assets

- [ ] Navigate to the GitHub releases page: `https://github.com/juralabs/jura-archive/releases/tag/v0.9.0-rc.3`
- [ ] Confirm the following release assets are present and downloadable:

  | Asset | Platform | Architecture | Status |
  |-------|----------|--------------|--------|
  | `Jura.Trace_0.9.0_aarch64.dmg` | macOS | Apple Silicon (aarch64) | Verify download link works |
  | `Jura.Trace_0.9.0_x64-setup.exe` | Windows | 64-bit Intel/AMD (x64) | Verify download link works |
  | `Jura.Trace_0.9.0_x64.msi` | Windows | 64-bit Intel/AMD (x64) | Verify download link works |

- [ ] Confirm that the release notes on the GitHub page are present and readable.
- [ ] Confirm that SHA-256 checksums are listed in the release notes or available on request.

> **Note**: A Linux AppImage is not available for this release candidate. Do not offer Linux installation to pilot testers at this stage. If a tester uses Linux exclusively, defer their participation to the v1.0 release or contact them to discuss an alternative arrangement.

---

### 1.2 Documentation

- [ ] Open `docs/pilot-testing/pilot-tester-guide.md`. Confirm:
  - The version number on page 1 reads **0.9.0 Release Candidate**.
  - Section 3 (Download) shows the correct release URL: `https://github.com/juralabs/jura-archive/releases/tag/v0.9.0-rc.3`.
  - Section 11 (Known Limitations) accurately reflects the current build.
  - Contact details throughout read `paul@juralabs.org`.

- [ ] Open `docs/pilot-testing/test-script.md`. Confirm:
  - The structured session tasks align with the features available in v0.9.0-rc.3.
  - Section 1 correctly notes that Linux is not available for this RC.

- [ ] Decide how you will share the pilot-tester-guide.md with testers. Options:
  - Export as PDF and attach to the welcome email.
  - Share a direct link if the document is hosted.

  Chosen method: _______________________________________________

---

### 1.3 Test Files

- [ ] Prepare at least one sample JPEG image (500 KB – 5 MB) that testers can use if they do not have their own files ready. This file should be a genuine photograph — not AI-generated — so that a Standard analysis is likely to return an Authentic verdict.
- [ ] Confirm the sample file is available to share (attached to the welcome email or linked from a download page).

---

### 1.4 Feedback Channel

- [ ] Confirm `paul@juralabs.org` is monitored and will receive pilot feedback during the two-week test period.
- [ ] Set a calendar reminder for each follow-up touchpoint (see Section 3).

---

### 1.5 Pre-Distribution Sign-off

All items above complete? Sign off before proceeding.

```
Checked by:

Date:

Notes:
```

---

## 2. Per-Tester Distribution Steps

Repeat this section for each pilot tester. Record the tester's name, platform, and role before beginning.

### Tester details

```
Tester name:

Platform (macOS / Windows):

Role / persona type:

Date of distribution:
```

---

### 2.1 Welcome Email

- [ ] Use the email template in `docs/pilot-testing/rc-tester-email.md`.
- [ ] Before sending, customise the following fields in the template:
  - Tester name (salutation line).
  - Platform-specific download link (macOS DMG or Windows EXE/MSI).
  - Any role-specific note if relevant (see persona-specific questions in `test-script.md`).
- [ ] Attach or link the pilot-tester-guide.md (in whichever format you chose in Section 1.2).
- [ ] Attach the sample test file if one was prepared.
- [ ] Review the email for tone and accuracy before sending.
- [ ] Send the email. Record the date sent in the tracking table (Section 4).

---

### 2.2 Download and Install Support

- [ ] Include in the email (or a follow-up) the specific file the tester should download for their platform:
  - **macOS**: `Jura.Trace_0.9.0_aarch64.dmg` (258 MB — Apple Silicon; runs on Intel Macs via Rosetta 2)
  - **Windows EXE**: `Jura.Trace_0.9.0_x64-setup.exe` (297 MB — recommended for most users)
  - **Windows MSI**: `Jura.Trace_0.9.0_x64.msi` (alternative for managed or enterprise Windows environments)
- [ ] Remind the tester about the expected security warning for their platform:
  - **macOS**: Gatekeeper will block the unsigned build on first launch. Right-click → Open is the workaround. Full instructions are in Section 4 of the pilot-tester-guide.
  - **Windows**: SmartScreen will show "Windows protected your PC". Click More info → Run anyway. Full instructions are in Section 5 of the pilot-tester-guide.

> **If the tester is on a managed corporate device**: they may be unable to install unsigned software without IT approval. Flag this risk before distribution. Offer to provide a SHA-256 checksum for the installer on request.

---

### 2.3 Feedback Channel Confirmation

- [ ] Confirm the tester knows to send all feedback and bug reports to `paul@juralabs.org`.
- [ ] Note the tester's preferred contact method and any specific times they are available for a brief check-in call if needed.

```
Preferred contact:

Available times (if relevant):
```

---

### 2.4 Persona Note

Before the tester begins, confirm which persona type best describes their role. This determines which persona-specific questions to ask during the structured feedback session (see Section 8 of `test-script.md`).

- [ ] Museum / archive staff
- [ ] Journalist / fact-checker
- [ ] Content creator
- [ ] Researcher
- [ ] Other: _______________________________________________

Record the persona in the tracking table (Section 4).

---

## 3. Follow-Up Schedule

Use this schedule for each tester. Adjust dates based on the date you sent the welcome email.

### Day 1 — Installation Confirmation

**Goal**: confirm the tester has successfully installed the application and can launch it.

- [ ] Send a brief check-in email or message: "Were you able to install Jura Trace and get it running? Let me know if anything blocked you."
- [ ] If installation failed, diagnose the issue using the relevant platform guide (`docs/install-guides/macos-unsigned.md` or `docs/install-guides/windows-unsigned.md`) and offer direct support.
- [ ] Record installation status in the tracking table.

**Common issues at this stage:**
- macOS: Gatekeeper blocking the app (right-click → Open workaround not tried)
- macOS Sequoia: quarantine flag requiring `xattr -cr` command
- Windows: SmartScreen "Run anyway" not found (managed device with Group Policy restriction)
- Windows: WebView2 Runtime missing on Windows 10

---

### Day 3 — First Verification Check-in

**Goal**: confirm the tester has completed at least one verification and understands the basic workflow.

- [ ] Send a check-in: "Have you had a chance to try verifying an image? Curious whether the results made sense at first glance."
- [ ] If they have not yet started, remind them of the 30-minute structured session in `test-script.md` and offer to walk through it together on a call if helpful.
- [ ] Note any early feedback mentioned informally — record it in the tracking table.

---

### Day 7 — Structured Feedback Request

**Goal**: collect the structured feedback from the test session.

- [ ] Send a prompt: "If you have not yet worked through the test script, now is a great time — it only takes about 30 minutes. When you are done, send your notes (however rough) to paul@juralabs.org."
- [ ] If the tester has already completed the session, follow up on any issues or open questions they raised.
- [ ] Confirm you have received their structured feedback or a date when they expect to send it.
- [ ] Record feedback status in the tracking table.

---

### Day 14 — Final Impressions and Pilot Close

**Goal**: collect final impressions and formally close the tester's participation.

- [ ] Send a closing email or message thanking the tester for their time.
- [ ] Ask for any final thoughts, particularly on:
  - Whether they would use the tool in their daily work.
  - Anything they feel strongly about adding before public release.
- [ ] Remind them of the licence terms: the build is licensed under AGPL-3.0-or-later. Any use case that cannot comply with the AGPL's terms requires a commercial licence from `licensing@juralabs.org`.
- [ ] Ask whether they would like to be notified when v1.0 is released.
- [ ] Record the pilot close in the tracking table.

---

## 4. Tester Tracking Table

Update this table throughout the distribution and follow-up process.

| Name | Platform | Role / Persona | Date Sent | Install Confirmed | Day 7 Feedback | Final Feedback | Pilot Closed |
|------|----------|----------------|-----------|-------------------|----------------|----------------|--------------|
|      |          |                |           |                   |                |                |              |
|      |          |                |           |                   |                |                |              |
|      |          |                |           |                   |                |                |              |

**Column notes:**
- **Date Sent**: date the welcome email was sent
- **Install Confirmed**: Y / N / Partial (note platform issues if N or Partial)
- **Day 7 Feedback**: Received / Pending / Not responding
- **Final Feedback**: Received / Pending / Not responding
- **Pilot Closed**: Y / N

---

## Quick Reference — Platform Files

| Platform | Installer | Size | Notes |
|----------|-----------|------|-------|
| macOS (Apple Silicon) | `Jura.Trace_0.9.0_aarch64.dmg` | 258 MB | Also runs on Intel Macs via Rosetta 2 |
| Windows (EXE) | `Jura.Trace_0.9.0_x64-setup.exe` | 297 MB | Recommended for most Windows users |
| Windows (MSI) | `Jura.Trace_0.9.0_x64.msi` | — | For managed/enterprise Windows environments |
| Linux | Not available | — | Linux AppImage planned for v1.0 |

---

## Quick Reference — Common Tester Issues

| Issue | Platform | Resolution |
|-------|----------|------------|
| "App can't be opened" — Gatekeeper warning | macOS | Right-click the app → Open → Open in the dialog |
| "App is damaged and can't be opened" | macOS Sequoia | Run `xattr -cr /Applications/Jura\ Trace.app` in Terminal |
| "Windows protected your PC" | Windows | Click More info → Run anyway |
| "Run anyway" button not visible | Windows (managed) | IT approval needed; offer SHA-256 checksum |
| Blank application window | Windows 10 | WebView2 Runtime not installed — see pilot-tester-guide.md Section 5 |
| Analysis engine shows Offline | All | Sidecar starting up — wait 15–20 seconds and refresh Settings |
| Forensic analysis not available | All | Analysis engine (Python sidecar) not yet running. In this RC, it must start automatically on launch. If it does not, ask the tester to quit and relaunch the application. |

---

*Distribution coordinator: Paul Griffiths, Juralabs Community Interest Company (UK) — paul@juralabs.org*
