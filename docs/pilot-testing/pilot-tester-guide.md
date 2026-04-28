---
title: "Jura Trace — Pilot Tester Guide"
description: "Everything a pilot tester needs to download, install, and evaluate Jura Trace, from first launch to giving feedback. Self-contained and written for non-technical readers."
last-updated: 28 March 2026
status: internal
---

# JURA TRACE — PILOT TESTER GUIDE

**Version 0.9.0 Release Candidate — March 2026**

---

Thank you for helping us test Jura Trace. You are among the first people outside Juralabs to use this tool, and your experience — what works, what confuses you, what is missing — will directly shape the product before it reaches a wider audience.

You do not need any technical background to take part. This guide walks you through everything step by step, from downloading the application to telling us what you think.

**Questions or problems at any point? Contact Paul Griffiths:**

> Email: paul@juralabs.org
> Response time: within 24 hours

---

## Contents

1. [What is Jura Trace?](#1-what-is-jura-trace)
2. [Compatible hardware](#2-compatible-hardware)
3. [Download](#3-download)
4. [Installation — macOS](#4-installation--macos)
5. [Installation — Windows](#5-installation--windows)
6. [First launch](#6-first-launch)
7. [Your first verification](#7-your-first-verification)
8. [Protecting content](#8-protecting-content)
9. [Exploring the help system](#9-exploring-the-help-system)
10. [How to give feedback](#10-how-to-give-feedback)
11. [Known limitations](#11-known-limitations)
12. [Reporting issues](#12-reporting-issues)

---

## 1. What is Jura Trace?

Jura Trace is a desktop application that helps you verify whether images, videos, and documents are authentic or AI-generated. It also helps you protect your own content by embedding cryptographic records and invisible watermarks.

In a world of synthetic media, knowing what is real matters. Jura Trace gives you a bedrock of trust to work from — 12 automatic forensic detectors, plus 3 on-demand investigation tools, that run entirely on your own computer. No images are sent to any server. No accounts are required. Nothing leaves your machine.

The application is built by **Juralabs Community Interest Company (UK)**, a social enterprise based in the United Kingdom. It is free for non-commercial use — for journalists, educators, museum staff, researchers, and NGOs. Paid tiers are available for commercial use, but you do not need them for this pilot.

This is a pre-release build. It is not yet signed with an official Apple or Microsoft certificate, which means your computer will show security warnings during installation. **This is normal and expected.** The steps in this guide explain exactly how to proceed past those warnings safely.

Jura Trace's tagline is *Know What's Real* — that is the purpose behind every feature you will test today.

---

## 2. Compatible Hardware

Before downloading, check that your computer meets these requirements.

### macOS

| Requirement       | Minimum                                      |
|-------------------|----------------------------------------------|
| Operating system  | macOS 13 (Ventura) or later                  |
| Processor         | Apple Silicon (M1, M2, M3, or M4) — recommended; Intel Macs are supported via Rosetta 2 |
| RAM               | 4 GB minimum; 8 GB recommended               |
| Free disk space   | 500 MB                                       |

**Not sure which processor you have?** Click the Apple menu in the top-left corner and choose **About This Mac**. If the chip shows "Apple M1", "M2", "M3", or "M4", you have Apple Silicon. If it shows "Intel Core", you have an Intel Mac.

Both work. Download the same file — the Apple Silicon build runs on Intel Macs automatically using Rosetta 2, a built-in compatibility layer in macOS.

### Windows

| Requirement       | Minimum                                          |
|-------------------|--------------------------------------------------|
| Operating system  | Windows 10 (version 1803 or later) or Windows 11 |
| Processor         | 64-bit Intel or AMD (x86-64) — not ARM           |
| RAM               | 4 GB minimum; 8 GB recommended                   |
| Free disk space   | 500 MB                                           |
| WebView2 Runtime  | Included with Windows 11; may need installing separately on Windows 10 (see [Section 5](#5-installation--windows)) |

---

## 3. Download

Download Jura Trace from the releases page:

```
https://github.com/juralabs/jura-archive/releases/tag/v0.9.0-rc.3
```

Choose the file for your operating system:

| Your computer | File to download                              | Size   |
|---------------|-----------------------------------------------|--------|
| macOS         | `Jura.Trace_0.9.0_aarch64.dmg`               | 258 MB |
| Windows       | `Jura.Trace_0.9.0_x64-setup.exe`             | 297 MB |

Save the file somewhere easy to find, such as your Downloads folder.

---

## 4. Installation — macOS

Follow these steps in order. The security warning in step 6 is expected — this is macOS protecting you from software that is not yet officially signed. Signed and notarised builds will be available from the public release (v1.0) onwards.

**Steps:**

1. Locate the downloaded `.dmg` file in your Downloads folder and double-click it to open it.

   [Screenshot: macOS Finder showing the Jura.Trace_0.9.0_aarch64.dmg file in the Downloads folder]

2. A window opens showing the Jura Trace application icon and an Applications folder shortcut.

   [Screenshot: DMG window showing the Jura Trace application icon with an arrow pointing to the Applications folder shortcut]

3. Drag the **Jura Trace** icon onto the **Applications** folder shortcut. Wait for the copy to complete.

4. Once copying finishes, close the DMG window. You can eject the disk image by pressing **Cmd + E** or dragging it to the Trash.

5. Open **Finder**, then click **Applications** in the left sidebar.

6. Find **Jura Trace** in the list. **Right-click** on it (or hold Control and click if you do not have a right mouse button).

   [Screenshot: Finder Applications folder with Jura Trace visible, right-click context menu open]

7. Select **Open** from the menu that appears.

8. A dialog will appear saying the app is from an unidentified developer. Click **Open**.

   [Screenshot: macOS Gatekeeper dialog saying "Jura Trace is from an unidentified developer" with the Open button highlighted]

9. Jura Trace will launch. You only need to do steps 6–8 once. After that, you can open the application normally from Finder, your Dock, or Spotlight.

---

### If you see "Jura Trace is damaged and can't be opened"

This message can appear on macOS Sequoia (version 15) with stricter security settings. It does not mean the file is damaged — it means macOS has quarantined it. You can clear this with one command.

1. Open **Terminal**. You will find it in **Applications** → **Utilities** → **Terminal**.

2. Click into the Terminal window, type the following command exactly as shown, and press **Return**:

   ```bash
   xattr -cr /Applications/Jura\ Trace.app
   ```

3. No output means it worked. Try opening Jura Trace again from Finder using the right-click method in steps 6–8 above.

---

### If you prefer to use System Settings instead of the right-click method

On macOS Sequoia, an alternative route is available:

1. Try to open Jura Trace normally (double-click). It will be blocked.
2. Open **System Settings** → **Privacy & Security**.
3. Scroll down to the Security section. You will see a message: "Jura Trace was blocked from use because it is not from an identified developer."
4. Click **Open Anyway**.
5. Authenticate with your password or Touch ID when prompted.

---

**Still stuck after trying these steps?** Email **paul@juralabs.org** with your macOS version (Apple menu → About This Mac) and a screenshot of the error message. You will receive a response within 24 hours.

---

## 5. Installation — Windows

Windows will show a security warning because the installer has not yet been submitted for code signing. This does not mean the file is harmful. The steps below explain how to proceed past the warning.

**Steps:**

1. Locate the downloaded `Jura.Trace_0.9.0_x64-setup.exe` in your Downloads folder.

2. Double-click the file to run the installer.

3. Windows SmartScreen will display a blue warning screen saying **"Windows protected your PC"**.

   [Screenshot: Windows SmartScreen dialog reading "Windows protected your PC" with the "More info" link visible at the bottom]

4. Click **"More info"** — this is a small link at the bottom of the SmartScreen dialog.

5. A **"Run anyway"** button will appear. Click it.

   [Screenshot: Windows SmartScreen dialog after clicking "More info", showing the "Run anyway" button]

6. If Windows User Account Control asks whether you want to allow the installer to make changes to your device, click **Yes**.

7. Follow the on-screen installation steps. The default settings are correct for most users.

8. When the installation finishes, Jura Trace will appear in your **Start menu**.

---

### Windows Firewall prompt

When you first launch Jura Trace, Windows Firewall may ask whether to allow the application's analysis service to communicate on your network. This service is a local forensic engine that runs entirely on your computer — it does not connect to the internet.

When prompted:

- Select **"Allow access on private networks only"**.
- Do not tick "Public networks".

If you accidentally click **Cancel** or **Block**, the forensic analysis features will not work. To fix this:

1. Open the **Start menu** and search for **"Windows Defender Firewall"**.
2. Click **"Allow an app or feature through Windows Defender Firewall"**.
3. Click **"Change settings"**, then find **Jura Trace** in the list.
4. Tick the **Private** checkbox and click **OK**.

---

### WebView2 Runtime (Windows 10 only)

Jura Trace uses the Microsoft Edge WebView2 Runtime to display its interface. Windows 11 includes this automatically. On Windows 10, it may not be installed.

**How to check:** launch Jura Trace. If the application window is blank or does not open, WebView2 is missing.

**To install it:**

1. Go to this address in your web browser:
   ```
   https://developer.microsoft.com/en-us/microsoft-edge/webview2/
   ```
2. Download the **Evergreen Standalone Installer** (choose the "x64" version).
3. Run the installer and follow the prompts.
4. Restart Jura Trace.

---

### If your organisation blocks unsigned software

Some workplaces use Group Policy to prevent unsigned software from running. If the **"Run anyway"** button does not appear, you are on a managed device and cannot bypass this restriction yourself.

Contact your IT department and ask them to allow `Jura.Trace_0.9.0_x64-setup.exe`. Let them know the file was provided directly by Juralabs Community Interest Company (UK) for evaluation purposes. A SHA-256 checksum for the file is available from paul@juralabs.org on request.

---

**Still stuck?** Email **paul@juralabs.org** with your Windows version and a screenshot of any error message. Your Windows version is shown in **Settings** → **System** → **About** → "Windows specifications".

---

## 6. First Launch

When Jura Trace opens for the first time, you will see a brief setup check that tells you what the application can do on your computer.

**Steps:**

1. Open Jura Trace. On macOS, find it in Applications or click its Dock icon. On Windows, find it in the Start menu.

2. Wait a moment for the application to load. You may see an onboarding overlay or a status panel on first launch.

   [Screenshot: Jura Trace main window on first launch, showing the onboarding setup screen]

3. The setup check shows the status of each optional component. Here is what each status means:

   | Status indicator | What it means |
   |---|---|
   | Green — Online | This component is ready and working |
   | Amber — Starting | The component is still loading. Wait a moment and it will turn green. |
   | Grey — Not installed | This is an optional component. The application still works without it. |

4. You will likely see the following statuses on first launch:

   - **Analysis engine** — should show green. If it shows amber, wait 10–15 seconds and check again.
   - **Video and audio** — may show "FFmpeg not installed". This is optional. Without it, Jura Trace can still analyse images fully. To enable video analysis:
     - **macOS:** open Terminal and run `brew install ffmpeg` (requires [Homebrew](https://brew.sh) — if you don't have Homebrew, paste `/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"` into Terminal first)
     - **Windows:** download FFmpeg from [gyan.dev/ffmpeg](https://www.gyan.dev/ffmpeg/builds/) (essentials build), extract to a folder, and add the `bin` folder to your system PATH
   - **Speech transcription** — will download a small language model (approximately 150 MB) the first time you use it. This is normal and happens in the background.
   - **AI descriptions** — shows whether Ollama is installed. This is entirely optional. You do not need it for any of the testing tasks in this guide.

5. Click through the onboarding screens to reach the main interface. You can revisit these settings at any time in **Settings** (found in the navigation bar).

---

> **Optional: enabling video analysis on macOS**
>
> If you would like to test video verification, Jura Trace requires a free tool called FFmpeg. To install it, open **Terminal** (Applications → Utilities → Terminal) and type:
>
> ```bash
> brew install ffmpeg
> ```
>
> If you have not used Homebrew before, you can install it first from https://brew.sh. This step is entirely optional — image analysis works without it, and all the core testing tasks in this guide use images only.

---

## 7. Your First Verification

This is the most important thing to try. Verifying an image takes less than a minute and gives you a clear sense of what Jura Trace does.

**What you need:** any JPEG or PNG image file on your computer. This could be a photo you have taken, something downloaded from the web, or one of the sample images at the bottom of this section.

**Steps:**

1. Click **VERIFY** in the navigation bar at the top of the application.

   [Screenshot: Jura Trace main window with the VERIFY section selected in the navigation bar]

2. The Verify page shows a drop zone in the centre of the screen. Drag your image file onto the drop zone, or click the drop zone to browse for a file.

   [Screenshot: Verify page drop zone before any file is loaded]

3. Jura Trace will begin analysis automatically. In Standard mode, this takes approximately 10–20 seconds.

   [Screenshot: Verify page showing the analysis progress indicator while a file is being processed]

4. When analysis completes, you will see:

   - **Verdict** — one of three results:
     - **Authentic** (shown in green) — the image shows consistent signals of genuine origin
     - **Inconclusive** (shown in amber) — the signals are mixed; the image may be authentic or modified
     - **Synthetic** (shown in red) — multiple signals indicate AI generation or significant manipulation
   - **Trust Score** — a number from 0 to 100. A higher score means more signals point towards authentic content.
   - **Detector results** — expandable sections showing what each forensic detector found

   [Screenshot: Verify results panel showing the Verdict label, Trust Score, and the list of collapsed detector sections]

5. Click on any section heading in the results to expand it and read more detail.

6. Click the **"?"** icon next to the Trust Score label. This opens an explanation of how the score is calculated. Jura Trace aims to be transparent about its methods — everything it does is explained in plain language.

---

### Things to try

Once you are comfortable with the basic verification, try these variations:

**Try an AI-generated image.** Download any image from an AI image generator (Midjourney, DALL-E, Adobe Firefly, or similar) and verify it. AI-generated images often score Synthetic, though the result depends on the generator and the image content. Results are not guaranteed — the system is designed to assist your judgement, not replace it.

**Try different investigation modes.** At the top of the Verify page, you will find a mode selector:

| Mode      | Analysis depth             | Approximate time |
|-----------|----------------------------|------------------|
| Standard  | Core detectors             | 10–20 seconds    |
| Deep      | All detectors including the four regional detectors (Segmented ELA, Shadow Consistency, Colour Temperature, Splice Boundary) | 30–60 seconds    |

Try the same image in both Standard and Deep modes and compare the results.

**Try your own photograph.** A photo taken on your phone or camera should score Authentic. If it scores Inconclusive, this may be because heavy JPEG compression, social media re-encoding, or editing has altered its forensic signature. This is useful feedback — please note it.

**Try the "?"  help icons.** These appear throughout the Verify results. Each one opens a plain-language explanation of the forensic technique being described. We want to know whether these explanations are clear.

---

> **A note on what the verdict means**
>
> A verdict of Authentic does not guarantee an image is real. A verdict of Synthetic does not guarantee an image is AI-generated. Jura Trace reports what its forensic signals detect. Like any analysis tool, it can be wrong. Use it as one layer of evidence alongside your own judgement, not as the sole basis for a conclusion.
>
> Click **Help** → **How Analysis Works** inside the application for a full explanation of each detector and its known limitations.

---

## 8. Protecting Content

The Protect section lets you add forensic records to your own images and documents — so that others can later verify they came from you.

This section is optional for your pilot testing. Try it if you have a few minutes, but focus on Verify first.

**What you need:** a JPEG image of your own — a photograph or a document scan works well.

**Steps:**

1. Click **PROTECT** in the navigation bar.

2. Drag an image onto the import area, or click to browse for a file.

   [Screenshot: Protect page showing the import drop zone before any file is added]

3. Your image appears in the asset list. Click on it to select it.

4. **Sign with C2PA provenance:**

   C2PA (Coalition for Content Provenance and Authenticity) is an open industry standard for embedding provenance records in media files. Think of it as a digital certificate of origin — a verifiable record of who created or handled the file, and when.

   - Find the **"Sign with C2PA"** section and enter your name or your organisation's name in the field provided.
   - Click **Sign with C2PA**.
   - Wait a moment for the process to complete. A confirmation message will appear.

   [Screenshot: Protect page showing the C2PA signing section with the name field and Sign button]

5. **Embed an invisible watermark:**

   The invisible watermark is a frequency-domain mark that survives common processing — JPEG compression, resizing, and moderate cropping. It is not visible to the naked eye. It carries a unique identifier that can be extracted later during verification.

   - Find the **"Invisible Watermark"** section.
   - Set the strength selector to **Medium**. (Medium is recommended for most uses — it balances invisibility with robustness.)
   - Click **Embed Watermark**.
   - Wait for the process to complete.

   [Screenshot: Protect page showing the watermark section with the strength selector set to Medium]

6. The protected file is saved automatically. The application will show you where it was saved.

7. To confirm the watermark worked, go to **VERIFY** and run analysis on the protected file you just created. Look for the **Watermark Detection** section in the results — it should show the watermark was found.

---

## 9. Optional: Setting Up Ollama (AI Descriptions and Claim Verification)

Ollama is a free tool that runs AI models locally on your computer. Jura Trace uses it for two optional features:

- **Image descriptions** — automatically generates a description of what is in an image (useful for cataloguing)
- **Claim verification** — checks factual claims in transcribed speech against a knowledge base

**You do not need Ollama for core verification.** The 12 automatic forensic detectors work without it. But if you would like to try the AI features, follow these steps.

### Step 1: Install Ollama

**macOS:**
1. Go to [ollama.com](https://ollama.com)
2. Click **"Download for macOS"**
3. Open the downloaded file and drag Ollama to your Applications folder
4. Open Ollama from Applications — it will appear as a small icon in your menu bar

**Windows:**
1. Go to [ollama.com](https://ollama.com)
2. Click **"Download for Windows"**
3. Run the installer
4. Ollama will run in the background (you will see a small icon in the system tray)

### Step 2: Download the Two Models

Open **Terminal** (macOS) or **Command Prompt** (Windows) and run these two commands:

```
ollama pull llava:7b
```

Wait for it to finish downloading (~4.7 GB). Then run:

```
ollama pull qwen2.5:7b-instruct
```

Wait for it to finish downloading (~4.7 GB).

**What these models do:**
- `llava:7b` — looks at images and generates descriptions (e.g., "A photograph of a coastal landscape at sunset")
- `qwen2.5:7b-instruct` — reads text and verifies factual claims against a knowledge base

### Step 3: Verify Ollama is Running

1. Open Jura Trace
2. Go to **Settings**
3. Look at **Service Status** — Ollama should now show **"Online"** with a green indicator
4. If it still shows "Offline", check that Ollama is running (look for its icon in the menu bar or system tray)

### Step 4: Test the AI Features

1. Go to **VERIFY** and drop an image
2. After analysis completes, look for the **AI Description** section in the results (if present, Ollama is working)
3. For claim verification, try verifying a video that contains speech — the transcribed text will be checked against the knowledge base

### Troubleshooting

- **"Ollama is not running"** — open the Ollama application. On macOS, find it in Applications. On Windows, check the system tray.
- **Model download is slow** — the models are ~4.7 GB each. On a slow connection, this can take 30-60 minutes. You can continue using Jura Trace while they download.
- **Not enough disk space** — you need approximately 10 GB free for both models. If space is tight, you can install just `llava:7b` (image descriptions) and skip `qwen2.5:7b-instruct` (claim checking).
- **Not enough RAM** — the models need approximately 8 GB of RAM. If your computer has only 4 GB, Ollama may run slowly or not at all. This is fine — skip Ollama and use Jura Trace's core features.

### Settings Reference

The default Ollama settings in Jura Trace are:

| Setting | Default Value | What It Does |
|---|---|---|
| Ollama URL | `http://localhost:11434` | Where Ollama runs (do not change unless told to) |
| Vision Model | `llava:7b` | Model for image descriptions |
| Text Model | `qwen2.5:7b-instruct` | Model for claim verification |

You can change these in **Settings** → **AI Assistant (Ollama)** if needed, but the defaults are correct for most people.

---

### Testing Ollama Features — Quick Reference

Once Ollama is installed and showing Online in Settings, use this table to test each feature.

| Feature | How to test | What to look for |
|---|---|---|
| **Text reading** | Verify a screenshot → click "Read Text (Ollama)" in the results | Extracted text appears in a monospace box below the button |
| **Claim checking** | Verify a video with speech using Deep mode | Transcript section in results, with each claim labelled Supported, Disputed, or Unverifiable |
| **Image description** | Verify any image with Ollama running | "AI Description" section appears in the results panel |

**Quick test — try this right now:** take a screenshot of this guide page, save it as a PNG, then drop it on the Verify page and click "Read Text (Ollama)." You should see the text from this guide extracted by the AI.

**If the button does not appear:** check that Ollama shows Online in Settings → Service Status, and that you have downloaded the `llava:7b` model (see Step 2 above).

---

## 10. Exploring the Help System

Jura Trace includes full in-app documentation. We want to know whether it is useful and whether anything is confusing or missing.

**To access Help:** look for **Help** in the navigation bar. On smaller windows it may be in a secondary menu.

The Help section contains these pages:

| Page | What it covers |
|---|---|
| How Analysis Works | All 12 automatic forensic detectors plus 3 on-demand investigation tools, in plain language, with their strengths and limitations |
| Glossary | Definitions of technical terms (C2PA, ELA, perceptual hash, and more) |
| Usage Guides | Practical workflows for different roles — museum archivists, journalists, researchers |
| IT and Compliance | A summary of how Jura Trace handles data, for IT teams and data protection officers |

**We specifically want to know:**

- Is the language in the Help pages clear? Or does it assume too much technical knowledge?
- Are there terms in the Verify results that you did not understand and could not find in the Glossary?
- Would the Usage Guides help you explain this tool to a colleague or manager?
- Is there anything about how the application handles your data that you would want to know but could not find?

---

## 11. How to Give Feedback

Your feedback is what makes this pilot worthwhile. There is no wrong answer and no bad observation. If something confused you, that confusion is useful data.

There are three ways to tell us what you found.

---

### Option A — In-app feedback (easiest)

1. Click **"Feedback"** in the bottom-left corner of the application.
2. Select the type of feedback: a bug, something confusing, or a feature idea.
3. Describe what happened in your own words.
4. Click **"Copy to Clipboard"**.
5. Open your email client, start a new message to **paul@juralabs.org**, paste the copied text, and send it.

---

### Option B — Email directly

Send any feedback to **paul@juralabs.org**. Include:

- What you were trying to do when something went wrong or confused you
- What happened instead of what you expected
- Your operating system (macOS or Windows) and version
- A screenshot, if you can take one

There is no required format. A few sentences is enough.

---

### Option C — Structured feedback form

After your testing session, please answer these five questions. Copy them into an email and send your answers to **paul@juralabs.org**.

---

**Question 1: What went well?**

What did you find easy, clear, or satisfying to use? Was there a moment where the application did exactly what you expected?

```
Your answer:
```

---

**Question 2: What was confusing?**

Were there any moments where you did not know what to do next, or where a result did not make sense? What did you expect to happen, and what happened instead?

```
Your answer:
```

---

**Question 3: What is missing?**

Is there a feature you expected to find but could not? Is there information the results panel should show but does not? Is there anything about how the application explains itself that felt incomplete?

```
Your answer:
```

---

**Question 4: Would you use this in your work?**

Could Jura Trace fit into your existing workflow? If yes — where and how? If no — what would need to change?

```
Your answer:
```

---

**Question 5: Recommendation score**

On a scale of 1 to 10, how likely would you be to recommend Jura Trace to a colleague doing similar work?

(1 = would not recommend, 10 = would recommend without hesitation)

```
Score (1–10):

Reason:
```

---

## 12. Known Limitations

This is a pre-release build and some things are not yet complete. The list below sets honest expectations so that you can distinguish bugs from known limitations.

| Limitation | Details |
|---|---|
| Security warnings on installation | Expected for unsigned pre-release builds. Signed installers are planned for v1.0. |
| No auto-updates | New versions must be downloaded manually from the releases page. |
| Video analysis requires FFmpeg | FFmpeg is a free third-party tool. See [Section 6](#6-first-launch) for how to install it. Image analysis works without it. |
| AI descriptions require Ollama | Ollama is a free tool for running local AI models. Not needed for any core testing task in this guide. |
| Monitor tab — URL watchlist | The Monitor tab shows a URL watchlist interface. Background checking is partially implemented. Results may be inconsistent. This feature is planned for completion in v1.1. |
| "Check for Updates" in Settings | This button will show "up to date" regardless of the actual version. Auto-update functionality is not yet connected in this build. |
| Batch watermarking | The "Watermark All Images" function on the Protect page works, but the progress display may not always update in real time on Windows. |
| Windows ARM processors | Windows builds currently target x86-64 only. ARM Windows devices (Surface Pro X, Snapdragon X) are not supported in this release. |

If you encounter behaviour that is not in this list and seems wrong, please report it. We want to know about it.

---

## 13. Reporting Issues

If something is not working, we want to hear about it. Every report helps.

**Email paul@juralabs.org** with as much of the following as you can provide:

- A description of the problem — what you were doing, what you expected, and what happened instead
- The steps you took before the error appeared (for example: "I clicked Verify, dropped in a PNG, and clicked Deep mode — the application froze")
- Your operating system and version (macOS: Apple menu → About This Mac; Windows: Settings → System → About)
- Your Jura Trace version (shown in Settings)
- A screenshot of any error message
- The file you were trying to analyse, if you are comfortable sharing it

**Response time: within 24 hours.**

---

### If the application crashes

Stop using it and send an email immediately. If you see any data on screen that you did not expect — for example, analysis results for a file you did not submit — please note exactly what you saw and report it right away.

In either case, your data has not been transmitted anywhere. All analysis happens on your computer. There are no external servers involved.

---

### Checking your Jura Trace version

1. Open the application.
2. Click **Settings** in the navigation bar.
3. Look for the **version number** in the Settings panel. It should read `0.9.0-rc.3` or similar.
4. Include this number in any bug report.

[Screenshot: Settings page showing the version number field]

---

Thank you again for taking the time to test Jura Trace. Every piece of feedback you provide — however small it seems — makes the product better for the people who will rely on it.

**paul@juralabs.org — Response within 24 hours**

---

```
──────────────────────────────────────────────────────────
Jura Trace v0.9.0 Release Candidate
Developed by Juralabs Community Interest Company (UK)
juralabs.org | paul@juralabs.org

Licence: PolyForm Noncommercial 1.0.0
This pre-release build is provided for evaluation purposes only.
It must not be used for commercial work.

All processing happens on your device.
No data is sent to Juralabs or any external service.
──────────────────────────────────────────────────────────
```
