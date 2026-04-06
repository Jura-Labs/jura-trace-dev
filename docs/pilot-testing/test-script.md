---
title: "Jura Trace — Pilot Test Script"
description: "Structured first-use session guide for pilot testers. Covers installation, all four main workflows, and structured feedback collection. Estimated duration: 30 minutes."
last-updated: 31 March 2026
status: internal
---

# Jura Trace — Pilot Test Script

This script guides a facilitator and tester through a structured first-use session. The session takes approximately 30 minutes and covers installation, first launch, the four main workflows, and the help system. It is designed to be followed without prior knowledge of the application.

**Session purpose.** Understand where the application succeeds and fails in communicating its value to new users. Every observation — confusion, delight, unexpected behaviour — is useful data.

**For the facilitator.** Read tasks aloud or share this document with the tester. Do not explain how to complete a task before the tester attempts it. Record observations in the [RECORD] sections. Ask the [FEEDBACK] questions after the tester has attempted each task, not before. If a tester gets stuck for more than two minutes, note the point of failure and move on.

**For the tester.** Work through each numbered step in order. Think aloud where you can — describe what you expect to happen before you act. There are no wrong answers.

---

## Contents

1. [Pre-Test Setup](#1-pre-test-setup) — 5 minutes
2. [First Launch](#2-first-launch) — 5 minutes
3. [Protect Workflow](#3-protect-workflow) — 5 minutes
4. [Verify Workflow](#4-verify-workflow) — 10 minutes
5. [Monitor Comprehension](#5-monitor-comprehension) — 3 minutes
6. [Help System](#6-help-system) — 2 minutes
7. [Overall Feedback](#7-overall-feedback)
8. [Persona-Specific Questions](#8-persona-specific-questions)

---

## Tester Profile

Complete this section before beginning.

| Field                    | Response |
|--------------------------|----------|
| Tester reference number  |          |
| Date                     |          |
| Facilitator              |          |
| Platform                 | macOS / Windows / Linux (circle one) |
| OS version               |          |
| Architecture             | Apple Silicon / Intel x64 / ARM / x86_64 (circle one) |
| Persona type             | Museum/archive staff / Journalist / Content creator / Researcher / Other |
| Technical comfort level  | 1 (non-technical) — 5 (developer) |

---

## 1. Pre-Test Setup

**Estimated time: 5 minutes**

Before the session begins, confirm the tester has the correct installer file. Refer them to the appropriate installation guide for their platform.

### Platform-specific installation

**macOS**
Follow the guide at [`docs/install-guides/macos-unsigned.md`](../install-guides/macos-unsigned.md).
Key points to communicate in advance:
- The macOS build is not yet signed with an Apple Developer ID. This is expected for a pilot build. Signed, notarised macOS installers will be provided once an Apple Developer certificate is obtained.
- Gatekeeper will show a warning on first open. The guide explains exactly how to proceed.

**Windows**
Follow the guide at [`docs/install-guides/windows-unsigned.md`](../install-guides/windows-unsigned.md).
Key points to communicate in advance:
- From RC4 onwards, Windows builds are signed with a Microsoft Azure Trusted Signing certificate. Most testers should not see a SmartScreen warning. If one does appear (publisher reputation is still being established), click "More info" then "Run anyway" — the publisher name should confirm Juralabs.
- WebView2 must be installed. Most Windows 10/11 machines already have it. The guide includes a link to the standalone installer if needed.

**Linux**
Follow the guide at [`docs/install-guides/linux-requirements.md`](../install-guides/linux-requirements.md).
Key points to communicate in advance:
- The AppImage format is recommended for pilot testing. No system-level installation is required.
- Ubuntu 20.04 and earlier are not supported. Ubuntu 22.04 (Jammy) or later is required.
- The tester may need to mark the AppImage as executable before launching.

### Installation record

- [ ] Tester has downloaded the correct installer for their platform and architecture.
- [ ] Tester has read (or been walked through) the relevant installation guide.
- [ ] Installation completed without errors.

[RECORD] Did the installation complete without issues?

```
Result: Yes / No

If No — describe what happened:




At which step did the issue occur:

```

[RECORD] Time taken to complete installation (from opening the installer to first application launch):

```
Minutes:
```

---

## 2. First Launch

**Estimated time: 5 minutes**

### Steps

1. - [ ] Open Jura Trace. On macOS, double-click the application in Applications or wherever it was saved. On Windows, launch from the Start menu or desktop shortcut. On Linux, double-click the AppImage.

2. - [ ] Wait for the application to load. The main window should appear with the onboarding overlay visible.

   [Screenshot: Onboarding overlay on first launch]

3. - [ ] Read through the onboarding overlay. Work through each screen without skipping any steps.

4. - [ ] Close or complete the onboarding overlay to reach the main application interface.

5. - [ ] Navigate to **Settings** using the navigation bar at the top (or side, depending on window width).

6. - [ ] Locate the **Service Status** section within Settings.

   [Screenshot: Service Status panel in Settings]

7. - [ ] Observe the status indicators for Analysis Services and Database.

[RECORD] Are Analysis Services shown as Online or Offline?

```
Status: Online / Offline / Not visible

Notes:
```

[RECORD] Is the Database Location showing a valid path?

```
Result: Yes / No / Not visible

Path shown (copy exactly):
```

[RECORD] Are there any other status indicators shown, and what do they display?

```
Notes:
```

[FEEDBACK] "Was anything confusing about the first launch — the onboarding screens, the Settings page, or the application layout?"

```
Response:




```

---

## 3. Protect Workflow

**Estimated time: 5 minutes**

This section tests the core protection tools: C2PA provenance signing and invisible watermarking. The tester should use a JPEG image of their own, or use the sample file provided by your Juralabs pilot coordinator.

> **Note on test files.** For consistent results, use a JPEG image between 500 KB and 5 MB. Very small images (under 50 KB) may produce lower watermark confidence scores.

### Steps

1. - [ ] Navigate to **Protect** using the main navigation.

2. - [ ] Import an image using the file picker or by dragging and dropping a JPEG into the import area.

   [Screenshot: Protect page with image loaded]

3. - [ ] Locate the **C2PA Provenance** section. Read the description of what it does.

4. - [ ] Enter an institution or creator name in the relevant field (the tester's own name or organisation name is fine).

5. - [ ] Select **Sign with C2PA** (or the equivalent button label shown in the interface).

6. - [ ] Wait for the signing process to complete. A confirmation message or updated status should appear.

   [RECORD] Did C2PA signing complete successfully? Note any error messages shown.

   ```
   Result: Yes / No / Error

   Error message (copy exactly if shown):
   ```

7. - [ ] Locate the **Invisible Watermark** section.

8. - [ ] Confirm the strength selector is set to **Medium**. If not, change it to Medium.

9. - [ ] Select **Embed Watermark** (or the equivalent button label shown in the interface).

10. - [ ] Wait for the embedding process to complete.

    [RECORD] Did watermark embedding complete successfully?

    ```
    Result: Yes / No / Error

    Error message (copy exactly if shown):
    ```

11. - [ ] Note the location where the protected file was saved (the application should indicate this).

    [RECORD] Where was the protected output file saved?

    ```
    Path or folder:
    ```

[FEEDBACK] "Was the signing and watermarking process clear? Did you understand what each step was doing? What — if anything — would you change?"

```
Response:




```

[FEEDBACK] "Did you feel confident that your image had been protected after completing these steps? Why or why not?"

```
Response:




```

---

## 4. Verify Workflow

**Estimated time: 10 minutes**

This is the core forensic analysis workflow. Use the same image that was protected in Section 3, so the tester can see what a verified-authentic result looks like.

### Steps

1. - [ ] Navigate to **Verify** using the main navigation.

2. - [ ] Drop or import the protected image from Section 3 into the Verify interface.

   [Screenshot: Verify page with image loaded, before analysis begins]

3. - [ ] Confirm the analysis mode is set to **Standard**. If a mode selector is visible and a different mode is selected, switch to Standard.

4. - [ ] Start the analysis. Wait for all results to load. Analysis in Standard mode typically takes 15–30 seconds on most hardware.

   [Screenshot: Verify page with completed results]

5. - [ ] Locate the **Trust Score** and **Verdict** at the top of the results panel.

   [RECORD] What trust score was shown? (Record the number and the colour of the indicator if visible.)

   ```
   Trust score:
   Verdict shown (Authentic / Inconclusive / Synthetic):
   Indicator colour:
   ```

6. - [ ] Scroll through the results. Locate the forensic detail sections — these may be collapsed by default. Expand at least two sections to read their contents.

7. - [ ] Find the help link ("?") near the trust score or verdict label. Click it.

   [RECORD] Did the help link open a help page or panel?

   ```
   Result: Yes / No / Link not found

   What did the help content show?
   ```

8. - [ ] Find the **Signal Agreement** or equivalent section that lists each forensic detector and its finding. Review the list.

   [Screenshot: Signal Agreement or detector results list]

9. - [ ] Locate the **Investigate Further** panel or section. Note any privacy-related warning displayed there.

   [RECORD] Was a privacy warning displayed in the Investigate Further panel?

   ```
   Result: Yes / No / Panel not found

   What did the warning say (summarise or copy)?
   ```

[FEEDBACK] "Do you understand what the trust score means? Could you explain it to a colleague?"

```
Response:




```

[FEEDBACK] "Is the verdict label clear? If the result had shown 'Inconclusive' rather than what you saw — what would that mean to you in the context of your own work?"

```
Response:




```

[FEEDBACK] "Were there any parts of the results you found confusing or unhelpful?"

```
Response:




```

---

## 5. Monitor Comprehension

**Estimated time: 3 minutes**

The Monitor section tracks whether protected assets have appeared in AI training datasets or online. This task is specifically about checking whether the tester correctly understands the scope and limitations of this feature before they have used it.

### Steps

1. - [ ] Navigate to **Monitor** using the main navigation.

2. - [ ] Read the disclaimer or information banner at the top of the Monitor page. Take as much time as needed.

   [Screenshot: Monitor page with disclaimer banner visible]

3. - [ ] Do not start any monitoring task. This section is about comprehension only.

[FEEDBACK] "In your own words — what can the Monitor tab detect, and what can it not detect?"

```
Response:




```

[RECORD] Does the tester correctly understand both the capability and the limitation? Score Y for a substantially correct answer, N for a misconception, P for partially correct.

```
Score: Y / N / P

Specific misconception noted (if any):




Correct understanding demonstrated (if any):
```

---

## 6. Help System

**Estimated time: 2 minutes**

### Steps

1. - [ ] Navigate to **Help** using the main navigation (this may be in the footer or a secondary navigation area).

2. - [ ] Browse the available help sections. Identify the methodology or 'how it works' page.

3. - [ ] Read at least one entry in the methodology page that describes a forensic technique (Error Level Analysis, perceptual hashing, or similar).

4. - [ ] Find the **Glossary** within the Help section.

5. - [ ] Look up one term that appeared in the Verify results you reviewed in Section 4.

   [RECORD] Which term did the tester look up? Was it found in the glossary?

   ```
   Term looked up:
   Found: Yes / No
   ```

[FEEDBACK] "Is the help documentation useful? Does it explain things in a way that would help you use the application more confidently? What is missing?"

```
Response:




```

---

## 7. Overall Feedback

Ask each question in order. Give the tester time to think before recording their response. Do not prompt or lead.

---

**Q1.** "On a scale of 1 to 10, how likely would you be to recommend Jura Trace to a colleague doing similar work?"

```
Score (1–10):

Reason given:
```

---

**Q2.** "What was the best thing about the experience today?"

```
Response:




```

---

**Q3.** "What was the most frustrating thing?"

```
Response:




```

---

**Q4.** "If you could add one feature that isn't currently there, what would it be?"

```
Response:




```

---

**Q5.** "Would you use this tool in your daily work? Why or why not?"

```
Response:




```

---

## 8. Persona-Specific Questions

Ask only the questions relevant to the tester's persona type (recorded in the Tester Profile at the top of this document). If the tester's role spans more than one category, ask questions from both.

---

### Museum and archive staff

**Q6.** "Your organisation may have hundreds or thousands of digital assets to protect. How would the batch watermarking feature fit into your existing digitisation workflow — where in the process would you use it?"

```
Response:




```

**Q7.** "Jura Trace stores its database locally on the machine where it is installed. Would you need the database to be accessible from a shared network drive, or from multiple workstations? How critical is that to your team?"

```
Response:




```

[RECORD] Does the tester's workflow require multi-user or networked database access? This is a known limitation in the current version.

```
Requirement: Yes — critical / Yes — desirable / No

Notes:
```

---

### Journalists and fact-checkers

**Q8.** "When you receive an image from a source and need to verify it quickly — where in your existing verification process would Jura Trace fit? What would you do with the result?"

```
Response:




```

**Q9.** "The Investigate Further panel includes a reverse image search option. Before using it, the application displays a privacy warning. Did you notice that warning? Was it clear enough about what data leaves your machine and what does not?"

```
Response:




```

[RECORD] Was the tester aware of the privacy boundary between local analysis (no data transmitted) and the reverse image search (external request)?

```
Aware: Yes / No / Partially

Notes:
```

---

### Content creators

**Q10.** "Would you sign your work with C2PA provenance before every publication, or only for certain types of content? What would influence that decision?"

```
Response:




```

**Q11.** "The watermark strength selector offers three levels: Low, Medium, and High. Higher strength survives more aggressive processing but is marginally more visible. For images intended for social media — which strength would you choose, and why?"

```
Response:




```

[RECORD] Does the tester understand the trade-off between watermark robustness and perceptual quality?

```
Understanding: Yes / No / Partially

Notes:
```

---

### Researchers

**Q12.** "The Signal Agreement dashboard shows the finding from each individual forensic detector alongside the overall verdict. Would that level of detail help your analysis, or would it introduce too much complexity for the conclusions you need to draw?"

```
Response:




```

**Q13.** "The methodology page describes the technical basis for each detector. Is that level of detail sufficient if you needed to cite the tool or its findings in a research paper or report?"

```
Response:




```

[RECORD] Does the tester need additional methodological detail (e.g. peer-reviewed references, confidence interval data, false positive rates)?

```
Additional detail needed: Yes / No / Possibly

Notes:
```

---

## Session Close

Thank the tester for their time. Remind them:

- All files analysed and all data generated during the session remain on their machine. Nothing was transmitted to external servers.
- The pilot build they installed is licensed under PolyForm Noncommercial 1.0.0. It is for evaluation purposes only and must not be used for commercial work.
- Any bug reports or follow-up thoughts can be sent to their Juralabs pilot coordinator.

[RECORD] Any observations noted by the facilitator during the session that were not captured in the structured sections above:

```
Facilitator notes:




```

---

## Completion Checklist

Confirm the following before closing the session record:

- [ ] Tester profile section completed in full
- [ ] All [RECORD] fields completed or marked as not applicable
- [ ] All [FEEDBACK] questions asked and responses recorded
- [ ] Persona-specific questions completed for the correct persona type
- [ ] Session close notes completed
- [ ] Tester has been thanked and reminded of data handling and licence terms

---

*Jura Trace is developed by Juralabs Community Interest Company (UK). Pilot builds are licensed under PolyForm Noncommercial 1.0.0. For pilot coordinator contact details, refer to your onboarding communication.*
