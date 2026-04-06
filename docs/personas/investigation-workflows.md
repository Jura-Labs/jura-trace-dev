# Jura Trace — Investigation Workflows

**Document type**: Persona-based workflow analysis
**Version**: 1.0
**Date**: 28 March 2026
**Status**: Working document — Sprint 20 planning input

---

## Purpose

This document walks through four detailed, scenario-based investigation workflows for Jura Trace. Each workflow represents a realistic, high-stakes task that a practitioner in the field would need to complete under professional pressure.

The goals are threefold:

1. Validate that the current Jura Trace toolset supports real-world investigation workflows end to end.
2. Identify specific gaps — features, integrations, and UX affordances — that block or impede those workflows.
3. Generate a prioritised, consolidated gap analysis to inform post-v1.0 roadmap planning (Phase A and Phase B).

The four scenarios are grounded in actual investigative practice: source photo verification for publication, breaking news video verification, temporal misattribution of viral imagery, and bulk evidence authentication for legal submission.

---

## Scenario 1: Marcus Chen — Freelance Investigative Journalist

**Organisation**: The Guardian / BBC
**Task**: A source sends a photo of a politician at a meeting they deny attending. Marcus needs to verify authenticity before publication.
**Time pressure**: He has a publication window of 4 hours. His editor is waiting.
**Stakes**: Publishing a fabricated or manipulated image could end his career and expose the outlet to legal action. Not publishing a genuine image delays an important story.

### Context

The photo arrives via Signal. It shows a senior minister in an unannounced meeting with a known lobbyist. The politician's press office has already categorically denied the meeting took place. Marcus has a medium-confidence source who claims to have been in the room.

The photo is a JPEG, 2.1 MB, taken on what appears to be a smartphone. The metadata may have been stripped by Signal's compression. There are no visible watermarks. The image has not been found in any reverse image search Marcus has already run manually.

---

### Step-by-Step Workflow

**Step 1: Document the original before touching it**

Before importing to Jura Trace, Marcus saves the original file from Signal to a dedicated case folder on his local drive, noting the time of receipt, the source pseudonym, and the method of delivery. He does not open the image in any editing application.

He opens Jura Trace and navigates to the PROTECT tab. He runs a standard C2PA verification scan on the file to check whether a provenance manifest exists before any analysis. This is a one-second non-destructive read.

*Tool used*: C2PA verification (read-only, no signing).
*What it reveals*: No C2PA manifest present. This is expected — smartphone photos rarely carry C2PA credentials yet. It does not indicate manipulation; it confirms the absence of provenance metadata, which is itself information.

**Step 2: Import and run the Quick scan**

Marcus switches to the VERIFY tab and drags the file into the drop zone. He selects the Quick investigation mode (approximately 8–12 seconds) to get an initial triage verdict before investing time in a deeper run.

*Tool used*: VERIFY pipeline, Quick mode (ELA + EXIF anomaly detection + perceptual hash lookup).
*What it reveals*: The Quick scan returns an overall trust score. Marcus looks first at the verdict label — not the percentage. He wants Authentic, Inconclusive, or Synthetic. If Synthetic or strongly Inconclusive, he escalates immediately and contacts his editor. If Authentic, he treats this as a green light to proceed to deeper analysis, not a publication clearance.

**Step 3: EXIF metadata — read the camera's testimony**

The first thing Marcus examines in detail is the EXIF panel. A Signal-compressed image may have stripped most metadata, but traces often survive.

*Tool used*: EXIF metadata extraction with anomaly detection.
*What he looks for*:
- Make/Model: Does the camera make match the claimed source's known device? An iPhone 14 from a source who Marcus knows uses a Samsung S22 is a discrepancy worth noting.
- DateTimeOriginal vs. DateTimeDigitised: If they differ by more than a few seconds, the file may have been re-saved after original capture.
- GPS coordinates: If present, do they place the image in the claimed location?
- Software field: Does it reference any editing application (Adobe Photoshop, GIMP, Snapseed)? Editing software signatures in EXIF are not proof of manipulation, but they are significant flags.
- Anomaly score: The Jura Trace anomaly detection surfaces inconsistencies between fields — for example, a camera model that cannot have produced the claimed date, or a GPS altitude that contradicts the stated indoor meeting location.

*What it reveals in this scenario*: The GPS fields are absent (stripped by Signal). The Make/Model fields read "Apple iPhone 13". The DateTimeOriginal reads a date consistent with the claimed meeting period. No editing software is detected. The anomaly score is low. This is consistent with an authentic image, but it is not verification — it is the absence of a specific class of red flag.

**Step 4: Run Standard mode — engage the full ML pipeline**

Having found no red flags in Quick mode, Marcus escalates to Standard mode (approximately 45–90 seconds). This engages the full ML sidecar pipeline.

*Tool used*: Standard mode — ELA, noise analysis, copy-move detection, deepfake detection (GBM classifier), NPR, chromatic aberration, JPEG ghost, segmented ELA, shadow consistency, colour temperature, splice boundary.

*What he looks for*:

- **ELA (Error Level Analysis)**: In an authentic JPEG, the error levels should be broadly uniform across the image. Regions that have been composited from another image often show distinctly different error levels because they have been compressed separately. In this photo, Marcus looks specifically at the face of the politician and the lobbyist. If either face shows anomalously high or anomalously low error levels relative to the background, that is a meaningful signal.
- **Copy-move detection**: Has any region been cloned within the image — for example, to place one person in front of a background they were not in, by copying a background patch over evidence of a different setting?
- **Deepfake detection (GBM classifier)**: The trained GBM classifier extracts an 80-feature vector and produces a probability score for AI generation. Marcus looks at the raw score, not just the flagged/clean verdict. A score of 0.4 is not the same as a score of 0.08. He wants the number.
- **Splice boundary analysis**: The three-signal edge analysis (JPEG grid discontinuity, noise boundary, feathering artefacts) looks for evidence that a region of the image has been composited. If the politicians' faces show boundary artefacts that their hands do not, that asymmetry is significant.
- **Shadow consistency**: Do the shadows on both subjects come from the same light source and direction? A subject lit from the left composited onto a background lit from the right creates a physical impossibility. This detector is particularly powerful for detecting head-swaps or face composites.
- **Colour temperature analysis**: Does the colour temperature of the subjects match the ambient environment? Different cameras used to capture different source images often produce subtly different colour balances, which CIELAB segmentation can surface.

**Step 5: Regional analysis — scrutinise the specific claim**

The claim is specific: the politician was present in this room at this time. Marcus uses the regional analysis panel (segmented ELA with 8x8 grid) to examine specific areas of the image rather than relying solely on aggregate scores.

*Tool used*: Segmented ELA, splice boundary, shadow consistency (regional outputs).
*What he does*: He looks at the ELA grid overlaid on the image. He pays particular attention to:
- The grid cells covering the politician's face and body
- The boundary region between the subjects and the background
- Any area that visually appears to have sharper or softer focus than the surrounding image

If two or more regional detectors flag the same zone (a composite amplification signal), that is a stronger indication than a single detector flagging a different zone.

**Step 6: Visual inspection using the filter tools**

Marcus switches to the visual inspection tools. This is where his own expert eye supplements the algorithmic analysis.

*Tools used*: Greyscale, edge detect, high contrast, brightness/contrast, zoom to 200–400%.

*What he does*:
- **Edge detect**: Reveals hard boundaries between composited regions that are smoothed over in the normal view. A sharp hard boundary around a face that does not match the soft boundary of a photographed subject in a similar setting is a classic compositing tell.
- **High contrast**: Amplifies tonal differences. Can reveal cloning artefacts, re-saved JPEG grid artefacts, and luminosity mismatches between composited elements.
- **Zoom at 200–300%**: Examines the pixel-level texture around the politician's face, particularly near hair edges, collar, and ear boundary. AI-generated or composited faces often show specific artefacts at this scale — over-smooth skin texture, irregular hair rendering, or boundary smoothing.
- **Greyscale**: Removes colour information and examines luminance structure alone. Useful for assessing whether lighting direction is consistent across the full image.

**Step 7: LLaVA image description**

Marcus runs the LLaVA 7B description (Ollama, if available). He is not using this to verify authenticity — language model descriptions are not verification tools. He uses it for two purposes: to extract any legible text visible in the image (venue signage, documents on the table, screen displays), and to get a neutral description of the setting that he can compare against what his source told him.

*Tool used*: LLaVA 7B via Ollama (optional, available on his MacBook Pro).
*What it reveals*: The description mentions a conference room with a visible logo on the wall behind the subjects. Marcus notes this detail for external corroboration — if he can identify the venue from the logo and confirm the room exists, that independently supports the location claim.

**Step 8: Perceptual hash lookup**

Marcus runs a perceptual hash check against the Jura Trace local database. Because this is a new image from a confidential source, it will not match anything in his local database. He notes this as expected.

*What this step does not do*: It does not search the web for visually similar images. Marcus currently has to leave Jura Trace and use TinEye or Google Lens externally to check whether this image has appeared previously with different metadata. This is a manual, workflow-breaking step.

**Step 9: Synthesise the evidence and build the trust report**

Marcus opens the PDF trust report export. He fills in the analyst declaration modal: his name, "The Guardian — Investigation Unit", a case reference number, and the date.

*Tool used*: PDF trust report with analyst declaration (Sprint 20 feature).
*What he includes*:
- Trust score and verdict label
- Raw numerical scores for ELA, deepfake classifier, splice boundary, shadow consistency, colour temperature (the numbers matter — a score of 0.42 with a threshold of 0.40 is a different claim than 0.78 with a threshold of 0.40)
- EXIF metadata summary including all anomaly flags
- LLaVA image description transcript
- A free-text "Analyst notes" section (currently absent — he must annotate the PDF manually after export)

*What the report is used for*: As an internal verification record, not as a publication artefact. It goes to his editor and, if the story proceeds, to the outlet's legal team. It documents that reasonable technical diligence was performed.

**Step 10: External corroboration (outside Jura Trace)**

Marcus must now leave Jura Trace entirely for several steps that the tool does not support:
- Reverse image search (TinEye, Google Lens, Bing Visual Search, Yandex) to check for prior publication
- Geolocation research: matching visible architectural features, signage, and room layout against known venues
- Cross-referencing with other sources who might have been present
- Checking whether the politician's official diary shows any scheduled events that day

These steps are currently entirely manual and external.

---

### Gaps Identified — Scenario 1

**Gap 1.1: No integrated reverse image search**
Marcus must leave the application to use TinEye, Google Lens, or Yandex. A "Search for this image online" button on the VERIFY results panel — using a BYOK (bring-your-own-key) TinEye API integration — would allow reverse image search without cloud upload privacy concerns, as the search would be keyed to the perceptual hash rather than uploading the full image.

**Gap 1.2: No analyst annotations in PDF report**
The trust report exports the algorithmic analysis but provides no space for the investigator's own written observations, hypotheses, or contextual notes. These must be added by editing the exported PDF. This breaks the chain of custody — the additions are not part of the original signed report.

**Gap 1.3: No prior-image lookup against news agency databases**
The perceptual hash database is local only. There is no integration with AP Archive, Getty, Reuters, or other wire service archives to check whether this image matches any known authentic photograph on record.

**Gap 1.4: Raw score display requires deliberate navigation**
Marcus wants the raw numerical score (e.g. "ELA: 0.43, threshold: 0.40") visible at a glance, not buried inside a Details toggle. The summary view shows traffic lights. For a professional investigator, the raw score is the primary data. A "Technical view" preference that persists between sessions would remove repeated navigation.

**Gap 1.5: No provenance timeline**
When C2PA credentials are present, they can contain a chain of provenance events (capture, edit, re-sign). Jura Trace reads the latest manifest but does not display the full chain. For journalistic verification, knowing whether an image was re-signed 30 minutes after capture is significant.

**Gap 1.6: No geolocation assistance**
Nothing in Jura Trace helps Marcus cross-reference visible landmarks or architectural features with known locations. This is a separate discipline (OSINT geolocation), but even an EXIF GPS coordinate rendered as a map preview, or a prompt to open the coordinates in OpenStreetMap, would reduce context switching.

### Priority Feature Requests — Marcus Chen

1. BYOK reverse image search (perceptual hash query, not raw image upload) — Phase A
2. Free-text analyst annotation layer in trust report PDF — Phase A
3. Raw scores as default display option (persisted preference) — Sprint 20 or Phase A
4. GPS coordinate map preview in EXIF panel — Phase A
5. C2PA provenance chain timeline view — Phase B

---

## Scenario 2: James Waterfield — BBC Verify Senior Analyst

**Organisation**: BBC Verify
**Task**: Breaking news — an explosion video is being shared on X/Twitter, claimed to come from a named conflict zone in the current week. James must assess whether it is genuine footage from the claimed location and time, or recycled material from a different conflict.
**Time pressure**: 45 minutes before broadcast. A wrong decision in either direction (running false footage, or spiking genuine footage) has serious public interest consequences.
**Stakes**: The BBC's reputation for verification is a core institutional asset. A single broadcast of fabricated conflict footage would be a crisis-level incident.

### Context

The video is a 47-second MP4. It shows a large explosion, street debris, and civilian reactions. The caption on X claims it shows a specific city, with a specific neighbourhood name, on today's date. The tweet was posted 2 hours ago. It has 40,000 retweets. Three verified accounts have already reshared it.

James has the raw MP4, downloaded by a researcher via yt-dlp before the tweet can be deleted. He has opened Jura Trace and also has two other browser windows open: Google Earth (for geolocation), and a weather archive service.

---

### Step-by-Step Workflow

**Step 1: Immediate triage — format validation and metadata**

James drags the MP4 into the VERIFY tab. He does not run a full analysis yet. First priority is raw metadata.

*Tool used*: Video metadata extraction (FFmpeg/ffprobe via sidecar). Output: codec, resolution, FPS, duration, creation time, GPS if present, encoder string, bitrate.

*What he looks for*:
- **Encoder string**: Does the encoder (e.g. `video/mp4; codecs="avc1.4D0028"`) match what a typical smartphone camera produces? Certain encoders are diagnostic of specific capture environments. A screencapture encoder is immediately suspicious.
- **Creation time**: Is there a `com.apple.quicktime.creationdate` tag or equivalent? If the embedded creation time matches today's date, that is consistent with the claim. If it shows a date from two years ago, the temporal misattribution is immediate and strong.
- **Resolution and FPS**: Professional CCTV footage has distinct resolution signatures. Consumer smartphone footage has different compression profiles to news agency footage or screen-recorded social media content.
- **GPS**: Rarely present in social media downloads (platforms strip it), but if present, it either confirms or immediately contradicts the claimed location.

*What it reveals in this scenario*: The encoder string is `lavf58.76.100` — this is FFmpeg's lavf encoder, which strongly suggests the video was re-encoded using FFmpeg, not captured directly from a device camera. This is a significant flag. It does not prove manipulation, but professional re-encoding is a common workflow step when recycling archival footage. James notes this.

**Step 2: Frame extraction — visual inventory**

James runs the video frame extraction with "Standard" settings (6 evenly-spaced frames as thumbnails).

*Tool used*: Video frame extraction (`POST /video/frames`), producing 6 JPEG thumbnails.

*What he does*: He examines the frames as a rapid visual inventory.
- Does the visual content match the claimed location? He is looking for architectural signatures, vegetation, vehicle types, road markings, and signage that could be cross-referenced with the named neighbourhood.
- Are there any visible temporal or meteorological inconsistencies? If the claimed date had documented overcast conditions in that city, but the video shows clear blue sky, that is a contradiction.
- Does any frame contain legible text — shop signs, street signs, licence plates — that could identify the location?

*Time used*: Approximately 3 minutes, including visual cross-reference against a Google Earth screenshot of the named neighbourhood.

**Step 3: Video deepfake analysis — is the footage authentic?**

James runs the video deepfake analysis in Standard mode (6 frames, approximately 12 seconds). He is not primarily looking for AI generation here — an explosion in a conflict zone is unlikely to be AI-generated at current capability levels. He is looking for:

*Tool used*: Video deepfake analysis — per-frame scores, temporal consistency signals (noise drift, spectral drift, LBP drift), aggregate verdict.

*What he looks for*:
- **Temporal consistency signals**: A genuine video has coherent noise characteristics across frames. If noise drift or spectral drift is high, that indicates the video may have been assembled from multiple source clips — a common technique when recycling archival footage with a different event's audio dubbed in.
- **Per-frame scores**: If specific frames score anomalously high for AI generation, that may indicate synthetic overlays (e.g. a digitally added fire or explosion) superimposed on authentic footage.
- **Aggregate verdict**: An "Inconclusive" verdict here means the tool cannot confirm either way. James treats this as a null result, not as clearance.

*What it reveals in this scenario*: Temporal noise drift is elevated at frames 3 and 4. This suggests a potential edit point — the video may have been assembled from two separate clips. The aggregate deepfake score is low (0.12), confirming the footage is not AI-generated. But the assembly indicator is significant.

**Step 4: Audio metadata and transcription**

James extracts the audio metadata and runs faster-whisper transcription on the audio track.

*Tool used*: Audio metadata extraction (codec, sample rate, channels, bitrate) + faster-whisper transcription.

*What he looks for*:
- **Audio codec consistency**: Does the audio codec match what the video encoder would produce in a continuous capture? An AAC audio stream with a bitrate inconsistent with the video's expected capture environment is a flag.
- **Transcription**: Are the voices speaking the language spoken in the claimed location? Fatima's team uses this for language verification. James uses it more directly: does anyone in the video say anything that contradicts or confirms the claimed location or date? Are there radio or broadcast announcements audible in the background that can be cross-referenced?
- **Background audio**: The transcription sometimes captures partial text from background radio, television, or announcements. These can be searched.

*What it reveals*: The transcription captures a brief radio broadcast in the background. James notes the phrase and flags it for a researcher to cross-reference against broadcast archives.

**Step 5: Deeper frame analysis — escalate to Deep mode**

Having found the temporal discontinuity signal, James escalates to Deep mode (20 frames, approximately 40 seconds).

*Tool used*: Video deepfake analysis, Deep mode. Frame timeline with per-frame scores, classifier scores, and heatmaps.

*What he does*: He expands the frame accordion to inspect frames 3 and 4 — the frames around the suspected edit point. He looks at the per-frame signal breakdown: ELA contribution, noise variance, spectral features. He notes whether the transition between frames 2–3 shows visual discontinuity consistent with a cut between two different source clips.

*What it reveals*: Frame 3 shows a noise variance spike that is inconsistent with the frames before and after. The heatmap highlights the sky region as the primary source of anomaly. This is consistent with a sky replacement or a cut from one outdoor environment to another with different atmospheric conditions.

**Step 6: C2PA check and EXIF anomaly scan**

James checks the video's C2PA credentials (none present, as expected for user-generated social media content) and runs the full EXIF anomaly scan.

*Tool used*: C2PA verification (video/mp4), EXIF anomaly detection.

*What he finds*: The EXIF anomaly detector flags a discrepancy between the file's modification timestamp and the claimed capture date. The modification date is three days before the tweet was posted, but the video claims to be from today. This is consistent with the file having been stored and then uploaded — not captured and immediately shared. It does not prove misattribution, but it adds to the cumulative signal.

**Step 7: Perceptual hash check**

*Tool used*: Perceptual hash (aHash/dHash/pHash on representative frames).

*What he does*: He hashes the key frames and queries the local database. For BBC Verify, the local database contains hashes of known archived conflict footage previously assessed by the team. If any frame matches a known archived clip, the provenance is immediately resolved.

*What it reveals in this scenario*: No match in the local database. This means the specific clip has not been assessed by BBC Verify before. It does not mean the clip has never appeared elsewhere.

**Step 8: Signal synthesis and broadcast decision**

James has 8 minutes left. He reviews the cumulative evidence:

- FFmpeg re-encoding (re-encoding from unknown source)
- Elevated temporal noise drift at frames 3–4 (possible assembly from two clips)
- Sky region anomaly on frame 3 (possible atmospheric discontinuity or sky replacement)
- Modification date pre-dating claimed capture date by 3 days
- No C2PA credentials
- No perceptual hash match in the known-archive database

No single signal is conclusive. The aggregate is "strongly Inconclusive, with multiple independent indicators of potential misattribution." James recommends to the broadcast team: do not run this footage without independent corroboration of the claimed location and date from a second source.

*Tool used*: PDF trust report with analyst declaration, case reference for the broadcast decision record.

**Step 9: Export the verification record**

James exports the trust report, including:
- Analyst name and case reference (Sprint 20 feature)
- Video metadata summary
- Frame thumbnails with per-frame scores
- Temporal consistency signal breakdown
- EXIF anomaly flags
- His written synthesis (added manually to the PDF after export — this is the current workflow gap)

This record is stored in the BBC Verify case management system.

---

### Gaps Identified — Scenario 2

**Gap 2.1: No keyframe comparison mode**
James needs to compare two frames side by side at the suspected edit point (frames 2–3). There is no frame comparison mode in the current interface. He currently screenshots individual frames and uses an external image viewer.

**Gap 2.2: No waveform or audio visualisation**
The audio transcription is text-only. James cannot visually inspect the audio waveform to identify edit points, dubs, or unnatural discontinuities between segments. An audio waveform display — even a simplified one — would allow rapid visual identification of audio cut points independent of the transcription.

**Gap 2.3: No reverse video search**
The perceptual hash lookup is against a local database only. For BBC Verify, the most important question is "Has this footage appeared previously elsewhere?" There is no integration with any service that supports video-level reverse search (InVID/WeVerify, Bellingcat's tools, Google Video Search). This is the single most time-critical missing capability for James's workflow.

**Gap 2.4: No edit-point annotation**
When temporal noise drift flags a specific timepoint, James cannot place a named annotation ("suspected edit point at 18.4s") on the timeline. Annotations would allow him to build a structured evidence narrative within the tool rather than managing this externally.

**Gap 2.5: No encoder database lookup**
The encoder string `lavf58.76.100` is surfaced in the metadata but not interpreted. The tool does not explain that lavf is the FFmpeg libavformat encoder, or what its presence typically indicates. For non-specialist users at BBC Verify (James knows this, but junior analysts may not), the raw encoder string requires external research.

**Gap 2.6: PDF trust report does not include video frame thumbnails by default**
The frame thumbnails are visible in the UI but are not embedded in the PDF export. For a broadcast decision record, the frame-level evidence should be in the report, not only in the application.

**Gap 2.7: 45-minute workflows are impeded by mode escalation steps**
Moving from Quick to Standard to Deep requires manual mode re-selection and re-run. Under deadline pressure, a "Run full analysis now" single-action that executes all four modes sequentially and streams results incrementally as each mode completes would reduce cognitive overhead significantly.

### Priority Feature Requests — James Waterfield

1. Frame-level annotations (named markers at specific timestamps) — Phase A
2. Frame comparison view (side-by-side or overlay at suspected edit points) — Phase A
3. Video frame thumbnails in PDF trust report — Phase A
4. Audio waveform display with edit-point detection — Phase B
5. Encoder string interpretation (human-readable explanation of encoder type) — Sprint 20 or Phase A
6. Incremental analysis streaming (results populate as each mode completes, no manual escalation) — Phase B

---

## Scenario 3: Fatima Al-Rashid — Bellingcat-Style OSINT Fact-Checker

**Organisation**: Independent verification organisation
**Task**: A viral photo on Telegram shows severe flooding in a named city, claimed to be from this week. Fatima's team believes it is a real photo but from a different event — possibly from flooding in the same city four years ago, or from a completely different country.
**Time pressure**: Medium. The photo is going viral but has not yet been picked up by mainstream media. She has 2–3 hours.
**Stakes**: The misattribution is politically charged. The flooding is being used to criticise the current government. If she debunks it, she needs her debunking report to be airtight — because the political backlash against her organisation will be severe.

### Context

The photo is a JPEG, downloaded from the original Telegram post via her researcher's workflow. It shows a flooded street with visible building facades, vehicles partially submerged, and civilians wading through water. The caption claims a specific date from this week and a specific district name.

Fatima has already run a manual reverse image search on TinEye and Google Lens. TinEye returned zero results. Google Lens returned visually similar images of flooding — generic results, not a match. She suspects the image may have been slightly cropped or colour-adjusted to defeat hash-based reverse search. She is now turning to Jura Trace for deeper forensic analysis.

---

### Step-by-Step Workflow

**Step 1: Metadata extraction — establish the baseline**

Fatima imports the image into VERIFY and runs the EXIF metadata extraction immediately.

*Tool used*: EXIF metadata extraction and anomaly detection.

*What she looks for*:
- **DateTimeOriginal vs. DateTimeDigitised**: If DateTimeOriginal exists and does not match the claimed week, the misattribution is established. She is looking for this first because it is the cleanest possible signal.
- **Software field**: Has the image been processed through software that would have reset the date, such as Snapseed, Lightroom, or an online resize tool? Some automated re-upload pipelines strip metadata but leave Software artefacts.
- **GPS**: If GPS is present and the coordinates do not match the claimed city, that is a contradiction. If GPS is absent, that is expected — either stripped by Telegram or never recorded.
- **Camera make/model**: Does the EXIF report a model consistent with the image quality and the claimed region? A Samsung Galaxy S24 from a country where that model was not widely available in the claimed year would be anomalous.

*What it reveals in this scenario*: DateTimeOriginal is absent (stripped). The Software field reads "WhatsApp 2.23.24.78". This indicates the image passed through WhatsApp before being posted to Telegram — a common resharing pattern. WhatsApp compresses images and strips most EXIF data but sometimes leaves a software field. This tells Fatima the image has been shared through at least one messaging platform before reaching Telegram.

**Step 2: Perceptual hash — check for known variants**

Fatima runs the perceptual hash check against her organisation's local database.

*Tool used*: Perceptual hashing (aHash, dHash, pHash).

*What this does*: Her organisation maintains a local Jura Trace database of previously assessed images, including images from known misattribution cases. A perceptual hash match — even with high hamming distance, indicating a cropped or adjusted variant — would immediately flag this as a known recycled image.

*What it reveals*: No match in the local database. This image has not previously been processed by her organisation's Jura Trace instance. This is a null result, not a clearance.

**Step 3: Run Standard mode — look for manipulation indicators**

Fatima runs Standard mode. She is not primarily looking for AI generation — she believes this is a real photograph from a real flooding event. She is looking for evidence of manipulation (cropping, colour adjustment, splice) that might explain why the reverse image search failed to find an exact match.

*Tools used*: ELA, noise analysis, copy-move, JPEG ghost, chromatic aberration.

*What she looks for*:
- **JPEG ghost**: The JPEG ghost detector analyses double-compression artefacts — the signature of a JPEG that has been re-saved at a different quality level. If the image shows ghost artefacts consistent with re-compression, the image was not captured and uploaded in a single step. It was captured, saved, potentially edited and re-saved. The quality levels of the ghost can sometimes narrow down the processing chain.
- **ELA**: If the flooding in the foreground shows different error levels to the buildings in the background, a composite is possible. For a misattribution case, Fatima is less concerned about AI compositing and more about the possibility that a sky or background from a different photo has been swapped.
- **Chromatic aberration**: Chromatic aberration follows a specific radial pattern from the lens centre. If different regions of the image show inconsistent CA patterns, they may have been captured with different lenses — indicative of compositing.
- **Noise analysis**: Sensor noise characteristics are partially specific to camera models. Significantly different noise profiles in different regions of the image indicate different capture sources.

*What it reveals*: The JPEG ghost detector shows the image has been re-compressed twice — once at approximately Q85 and once at approximately Q60. The current file appears to have been at Q60. This is consistent with a photo that was originally a higher-quality capture, posted to social media (which typically re-compresses to around Q85), then downloaded and reshared (bringing it to Q60). The double-compression is expected for a viral social media image and does not itself indicate manipulation.

ELA shows broadly uniform error levels. No strong splice boundary is detected. The image does not appear to be composited.

**Step 4: Visual inspection — geolocation analysis**

Fatima switches to the visual inspection tools. This is where her OSINT expertise takes over.

*Tools used*: Zoom (200–400%), brightness/contrast, greyscale, edge detect.

*What she does*:
- **Zoom at 300%**: She examines specific regions of interest — the building facades in the background. She is looking for architectural details that can be cross-referenced with street-level imagery (Google Street View, Yandex Maps, Mapillary). Distinctive window shapes, balcony configurations, shop sign styles, and road furniture are all potential geolocation anchors.
- **Brightness/contrast boost**: Flooded street scenes often have low-contrast distant backgrounds. Boosting contrast reveals building details, signage, and street furniture that may be illegible in the original image.
- **Greyscale**: Strips colour information, allowing the luminance structure of buildings and surroundings to be examined more clearly for architectural matching.
- **Edge detect**: Clarifies the precise outline of architectural features, making structural matching against Street View imagery more reliable.

*What she finds*: She identifies a distinctive rooftop water tank configuration visible on a building in the upper-left quadrant of the image. This style of tank is architecturally specific to one region. She notes this as a geolocation anchor.

**Step 5: LLaVA description — text extraction**

Fatima runs LLaVA to extract any legible text from the image.

*Tool used*: LLaVA 7B via Ollama.

*What she is looking for*: Any visible text — shop signs, street signs, vehicle registration plates, billboard fragments — that could place the image geographically or temporally. Even a partial word in a local language, or a visible logo of a local business, can narrow the search.

*What it reveals*: LLaVA identifies partially legible text on a shop front in the background. The text appears to be in a regional script. Fatima notes this for a specialist colleague to translate and search.

**Step 6: RAG claim checker — cross-reference the temporal claim**

Fatima runs the RAG claim checker with the image's claimed caption as the query.

*Tool used*: Qwen2.5 7B via Ollama, RAG claim checker.

*What she inputs*: The caption text from the Telegram post, including the claimed date, district name, and city.

*What the claim checker does*: It queries the local RAG knowledge base (which covers digital rights, cultural heritage, media verification methodology, and video forensics) to assess whether the claim is internally consistent with the forensic findings. It does not search the web. It evaluates whether known patterns of misattribution apply.

*Limitation she encounters*: The RAG knowledge base does not contain historical weather or flooding records for the claimed city. The claim checker can tell her whether the forensic signals are consistent with a real, unmanipulated photograph — which they are — but it cannot tell her whether there was actual flooding in that city this week. This requires external research.

**Step 7: External cross-reference (outside Jura Trace)**

Fatima must now work outside Jura Trace for several critical steps:
- Weather archive cross-reference: She queries a weather archive service for the claimed city and date. If no significant rainfall was recorded this week but the photo shows deep flooding, the temporal claim is contradicted by meteorological data.
- Flood event database: She checks known flood event databases and NGO reporting for the claimed city and region.
- Yandex Reverse Image Search: She runs the image through Yandex, which uses a different hashing algorithm to Google and TinEye and sometimes finds matches they miss.
- Geographic cross-reference: She opens the annotated building detail in Google Earth and Street View to attempt to match the architecture.

*What she finds*: Weather records show no significant rainfall in the claimed city this week. The meteorological contradiction, combined with the architectural peculiarities she noted, leads her to focus her search on the same city's flooding history. She finds a news agency photograph from four years ago showing the same street, same building, same water tank configuration — from a major flooding event that year.

**Step 8: Confirm misattribution and document the evidence chain**

Fatima returns to Jura Trace. She imports the original archival photograph (from the news agency's public archive) into a new VERIFY session and runs a perceptual hash comparison between the two images.

*Tool used*: Perceptual hashing — she is checking whether the viral image and the archival photograph produce similar hash distances.

*What she finds*: The perceptual hash distance between the two images is high — confirming the images are not pixel-identical but are visually very similar. This is consistent with the viral image being a cropped or colour-adjusted version of the archival photograph.

**Step 9: Build the evidence chain report**

Fatima exports the trust report for both images (the viral image and the identified archival original), including:
- Analyst declaration (name, organisation, case reference)
- JPEG ghost analysis (showing double compression history of the viral image)
- Perceptual hash comparison results and hamming distance
- EXIF metadata summary for both images
- Raw numerical scores for all detectors
- LLaVA text extraction output
- A written methodology section (currently added manually outside Jura Trace)

The final debunking report combines the Jura Trace PDF with the external evidence (weather records, archival photo citation, geographic matching screenshots). This multi-source evidence package is what she publishes.

---

### Gaps Identified — Scenario 3

**Gap 3.1: No perceptual hash comparison between two images side by side**
Fatima currently runs two separate VERIFY sessions and mentally compares the hash outputs. There is no "Compare two images" mode that shows the hamming distance between two perceptual hashes and visualises the difference. This is the most important missing capability for misattribution investigation.

**Gap 3.2: No integrated weather or event cross-reference**
The claim checker queries a local knowledge base. For temporal misattribution, the most valuable external data is meteorological records (was there actually flooding on this date?) and event databases (has this flooding been reported elsewhere?). An opt-in external lookup — even a simple one that opens the relevant weather archive with the claimed coordinates and date pre-filled — would reduce context-switching significantly.

**Gap 3.3: JPEG ghost quality level output is not sufficiently granular**
The JPEG ghost detector returns a flag (double compression detected) and an approximate quality level, but does not produce the ghost artefact map visualised as an overlay. Seeing where in the image the ghost artefacts are strongest — and whether they are localised to specific regions — would help distinguish a uniformly re-compressed image (typical social media resharing) from a localised re-compression (indicative of a spliced region that was saved separately).

**Gap 3.4: No methodology versioning in exported reports**
Fatima's debunking reports need to be reproducible. If she runs the same image through Jura Trace six months later and gets a different score because the GBM classifier has been retrained, that is a methodological integrity problem. The exported PDF should include the version number of every model and algorithm used in the analysis. The CLAUDE.md references methodology versioning as a Phase A item. It should be treated as a Professional-tier minimum.

**Gap 3.5: Free-text metadata annotation absent**
Fatima annotates her findings extensively during analysis. Currently she must maintain a parallel notes document. An annotation panel — a simple free-text field associated with a specific session — that is exported as part of the trust report would close a significant workflow gap.

**Gap 3.6: Batch comparison not available**
When Fatima's team has identified a cluster of potentially recycled images (five images all claiming to show the same event), she cannot run a batch perceptual hash comparison to identify duplicates or near-duplicates within the set. This requires exporting all hashes and running comparison logic externally.

### Priority Feature Requests — Fatima Al-Rashid

1. Image comparison mode (two images, side-by-side hash distance and visual diff) — Phase A
2. JPEG ghost artefact map overlay on image — Phase A
3. Methodology and model version embedding in PDF export — Phase A (hard requirement for reproducibility)
4. Free-text analyst annotation field in VERIFY session, exported in PDF — Phase A
5. Batch perceptual hash comparison within a session or case — Phase B
6. Opt-in external lookup for weather/event cross-reference (pre-filled URL redirect, not a cloud call) — Phase B

---

## Scenario 4: Amara Osei — Human Rights Watch Digital Evidence Manager

**Organisation**: Human Rights Watch (field office, digital evidence team)
**Task**: A field team has collected 200 photographs over three months from a conflict zone. The photographs document alleged violations. Some may have been tampered with, some may have had metadata stripped in the field to protect sources. Amara needs to authenticate the entire collection for potential submission to the International Criminal Court.
**Time pressure**: The submission deadline is in six weeks. Authentication must be completed, documented, and reviewed by legal counsel before that date.
**Stakes**: Evidence not meeting the ICC's admissibility standards is excluded. Evidence that is later found to be manipulated — or whose chain of custody is incomplete — can be used to challenge the entire submission and undermine credibility.

### Context

The 200 photographs are a mix of JPEG, PNG, and HEIC formats. They were captured across multiple field operatives using different devices. Some were transmitted via Signal (which compresses and strips metadata). Others were transmitted on physical media (SD cards carried out of the zone). All have been collected into a single folder.

Amara has the entire collection on an encrypted external drive. She has never seen most of these photographs before today. She needs a structured, repeatable workflow that produces a complete authentication record — not a pass/fail judgment, but a documented assessment of the confidence level for each image and the reasoning behind it.

---

### Step-by-Step Workflow

**Step 1: Establish baseline documentation before any analysis**

Before any image is imported to Jura Trace, Amara's team creates SHA-256 checksums for every file in the collection using a separate tool (sha256sum on Linux). These checksums are recorded in a spreadsheet. This establishes the integrity baseline — she can prove that Jura Trace received the files in the same state they were collected.

This step is entirely external to Jura Trace. It is a pre-processing requirement she must complete manually.

*Gap identified immediately*: Jura Trace does not provide a bulk SHA-256 hash generation function for an input collection. This should be the first action in any chain-of-custody workflow and is currently absent.

**Step 2: Batch import and triage — run standard mode across all 200 images**

Amara uses the Protect page batch import to bring the entire collection into the Jura Trace database. She does not want to modify or protect the images — she wants to assess them. But the batch import function is currently on the PROTECT tab, not the VERIFY tab.

*Tool used*: Batch import (PROTECT tab, "Watermark All Images" mechanism).

*Problem encountered*: The batch function is designed for watermark application, not for batch verification. Amara does not want to watermark the evidence — she wants to verify it. She must import each image individually through VERIFY, or use the PROTECT tab's import to build the database and then query each image.

*What she needs*: A batch VERIFY function that queues all 200 images, runs them through Standard mode, and produces a summary table showing the trust score, verdict, and key signal flags for every image. This is the most fundamental gap in her workflow.

*Workaround she uses*: She imports all 200 images via PROTECT (without watermarking), which at least builds the local database with hashes and metadata. She can then query each image for provenance. But she cannot run the full ML forensic pipeline in batch.

**Step 3: Individual verification — prioritise the outliers first**

Amara filters the PROTECT database by import date and identifies images that came via Signal (she can often tell from filename patterns and the WhatsApp/Signal software tags). She prioritises these for individual VERIFY analysis, as they are most likely to have stripped metadata.

*Tool used*: SQLite database query (indirectly, through the PROTECT tab's asset list).

*What she wishes she had*: A filter in the database view that shows EXIF anomaly flags, software tags, and metadata completeness indicators without requiring her to open each image individually.

**Step 4: EXIF analysis across the collection**

For each image she opens in VERIFY, the first stop is EXIF metadata.

*Tool used*: EXIF metadata extraction and anomaly detection.

*What she looks for*:
- **Metadata completeness**: Does this image have DateTimeOriginal, GPS, Make/Model? Missing metadata from a field collection is expected but must be documented — specifically, was it stripped in transit (Signal/WhatsApp) or was it never present (some older devices)?
- **Temporal consistency**: Do the timestamps across the collection form a coherent chronology? An image timestamped three months before the collection period began is either misdated or from a different event.
- **Cross-device consistency**: She groups images by claimed operative (based on metadata source) and checks whether the EXIF device information is consistent. If operative A's images are all from an iPhone 13, an image attributed to operative A that shows a different camera model is anomalous.
- **Anomaly flags**: Any EXIF anomaly flag — inconsistent field values, implausible timestamps, editing software in the chain — is recorded in her authentication log.

**Step 5: C2PA provenance check**

*Tool used*: C2PA verification on every image.

*What she finds*: Most images have no C2PA credentials — field documentation in conflict zones is rarely captured with C2PA-enabled devices. However, one cluster of images was captured by a journalist who used an Adobe-enabled camera. Those images carry C2PA manifests with verified timestamps and device signatures.

*What she does with this*: The C2PA-signed images are marked as highest-confidence in her authentication log. The cryptographically verified provenance chain is the strongest possible evidence of authenticity for ICC purposes.

*What she notes for the record*: The absence of C2PA credentials in the other images is not evidence of manipulation — it is the expected state for non-professional devices. She documents this explicitly.

**Step 6: Forensic analysis — full pipeline on flagged images**

For images that showed EXIF anomalies or where the provenance chain is unclear, Amara runs the full forensic pipeline in Deep mode (the most thorough available for still images).

*Tool used*: Deep mode — full ML pipeline including all 12+ detectors, regional analysis.

*What she looks for*:
- **ELA and segmented ELA**: Are there regions of anomalous compression in images that document specific violations? A photograph of an injury where the injury itself has different error levels to the surrounding body is a major red flag that must be documented and flagged for legal review.
- **Copy-move detection**: Has any element of the photograph been cloned or duplicated? For evidence photography, this would represent a form of fabrication.
- **Splice boundary analysis**: Has the background or foreground been replaced?
- **Shadow consistency and colour temperature**: Are all subjects in the photograph illuminated by the same light source? Physical inconsistency is indicative of compositing.
- **Deepfake detection**: Is the image AI-generated or does it contain AI-generated elements?

*What the output looks like*: For each image, she records the trust score, verdict label, and the raw numerical scores for every detector. She uses Jura Trace's "Details" view for the numerical breakdown, not just the traffic light summary.

**Step 7: Watermarking and chain-of-custody signing for the authenticated collection**

Having completed the forensic assessment, Amara now wants to sign the collection to establish a chain of custody. For images that have passed verification, she runs C2PA signing with Human Rights Watch's identity, adding provenance metadata: organisation, analyst name, verification date, methodology reference.

*Tool used*: C2PA signing (PROTECT tab), watermark embedding (optional).

*What she does*: She does not watermark the original evidence images (watermarking could be challenged as alteration). Instead, she creates a separate verified copy alongside each original, maintaining the originals in their unmodified state on the encrypted drive.

*What she wants*: A "Verify and sign" workflow that explicitly separates the original from the verified copy with a documented chain-of-custody record — rather than requiring her to manually manage original and copy in parallel.

**Step 8: Audit log review**

Amara reviews Jura Trace's audit log (the SHA-256 hash chain of all operations).

*Tool used*: Audit log with hash chain verification (`verify_audit_chain`).

*What the log provides*: A tamper-evident record of every operation performed in the Jura Trace session, including every image imported, every analysis run, and every signing operation. The hash chain allows independent verification that the log has not been modified after the fact.

*What she exports*: The audit log in full, as a supplementary document to the authentication report. For ICC submission purposes, the audit log demonstrates the procedural integrity of the verification process.

**Step 9: Produce the authentication report**

For each of the 200 images, Amara produces an individual trust report. For the ICC submission, these must include:
- Analyst declaration (name, organisation, role, case reference)
- SHA-256 hash of the original file (established in Step 1, manually inserted)
- EXIF metadata summary
- C2PA verification status
- Raw forensic scores for all detectors
- Verdict label with confidence indication
- Methodology reference (which version of Jura Trace, which model versions, which analysis mode)
- Analyst notes

*Problem*: Producing 200 individual PDF reports is impractical. She needs a bulk export that produces either a single comprehensive report covering the entire collection, or a machine-readable format (JSON, CSV) that her legal team's systems can ingest, with individual PDFs generated on demand for specific images being submitted.

*Tool used*: PDF trust report (individual, per image).

*Gap*: There is no bulk export function.

**Step 10: Berkeley Protocol compliance review**

The Berkeley Protocol on Digital Open Source Investigations (2020) sets standards for the documentation and preservation of digital evidence for international legal proceedings. Amara reviews her authentication workflow against the Protocol's requirements.

*What Jura Trace supports*: Chain-of-custody documentation via the audit log, metadata preservation, forensic analysis with documented methodology.

*What Jura Trace does not yet address*: The Berkeley Protocol requires documentation of the capture environment (the technical conditions under which the analysis was conducted), the analyst's qualifications and independence, and a specification of the limitations of the forensic tools used. None of these are currently part of the trust report template.

The in-app help system includes a Berkeley Protocol page (per the Sprint 19 work on help documentation), but there is no structured template in the PDF export that reflects the Protocol's evidence documentation requirements.

---

### Gaps Identified — Scenario 4

**Gap 4.1: No batch VERIFY function**
This is the most critical missing capability for Amara's workflow. Processing 200 images individually through the VERIFY tab is not feasible within a six-week authentication timeline. A batch VERIFY queue — with Standard or Deep mode, producing a summary table of results — is the single feature that would make Jura Trace viable for institutional evidence authentication at scale.

**Gap 4.2: No bulk SHA-256 hash generation for input collections**
The SHA-256 hash generation that exists for the audit log applies to operations within Jura Trace. There is no function to generate checksums for input files at the point of import, creating an external pre-processing requirement.

**Gap 4.3: No bulk PDF/JSON export for entire collections**
Producing 200 individual PDFs manually is impractical. A bulk export function — producing either a single collection-level report with per-image summaries, or a CSV/JSON export of all scores suitable for downstream processing — is required for institutional-scale workflows.

**Gap 4.4: No "Verify and archive" workflow that preserves originals**
The current PROTECT flow modifies files (adding C2PA metadata or watermarks). For evidence authentication, originals must be preserved unmodified. A workflow mode that explicitly creates authenticated copies while preserving originals — with a documented chain-of-custody linking original to authenticated copy — does not exist.

**Gap 4.5: Berkeley Protocol report template absent**
The PDF trust report does not include the structured fields required by the Berkeley Protocol: capture environment documentation, analyst qualifications, limitations disclaimer, protocol version reference. These must currently be added manually outside the tool.

**Gap 4.6: No database-level filtering by EXIF completeness or anomaly flag**
Amara cannot filter the asset database to show all images with EXIF anomalies, or all images with stripped metadata, without opening each image individually. A filter/sort capability on the database view — by anomaly flag, metadata completeness, trust score, or verification status — is essential for managing large collections.

**Gap 4.7: No cross-collection deduplication**
In a 200-image collection, some images may be duplicates or near-duplicates (the same scene photographed twice). Jura Trace does not identify near-duplicate images within a collection using perceptual hash distance. Cross-collection deduplication would help Amara identify when multiple operatives photographed the same event and allow her to confirm cross-device corroboration.

### Priority Feature Requests — Amara Osei

1. Batch VERIFY queue with progress tracking and summary table — Must Have for institutional adoption, Phase A
2. Bulk export (collection-level PDF report or JSON/CSV export of all scores) — Must Have for institutional adoption, Phase A
3. Input file SHA-256 hash generation at import — Must Have for chain-of-custody compliance, Phase A
4. Berkeley Protocol PDF report template (structured fields for legal proceedings) — Phase A
5. Database filter/sort by EXIF anomaly flag, metadata completeness, trust score — Phase A
6. "Verify and archive" mode (authenticated copy + preserved original, with documented chain) — Phase B
7. Cross-collection near-duplicate detection via perceptual hash distance matrix — Phase B

---

## Consolidated Gap Analysis

The following table compiles all gaps identified across the four scenarios. Each gap is assessed for which personas require it, the likely priority, and the phase in which it should be addressed.

### Gap Analysis Table

| Gap ID | Description | Personas | Priority | Phase |
|--------|-------------|----------|----------|-------|
| 1.1 / 3.1 | Perceptual hash comparison between two images (misattribution detection, visual diff) | Marcus, Fatima | Must Have | Phase A |
| 4.1 | Batch VERIFY queue (all modes, summary table output) | Amara, Fatima | Must Have | Phase A |
| 4.3 | Bulk export — collection-level PDF or JSON/CSV of all scores | Amara, Fatima | Must Have | Phase A |
| 4.2 | Input file SHA-256 hash at import (chain-of-custody baseline) | Amara | Must Have | Phase A |
| 1.2 / 3.5 | Free-text analyst annotations in VERIFY session, exported in PDF | Marcus, Fatima, Amara, James | Must Have | Phase A |
| 3.4 | Methodology and model version embedding in exported PDF | Fatima, Amara | Must Have | Phase A |
| 4.5 | Berkeley Protocol report template (structured legal evidence fields) | Amara | Must Have | Phase A |
| 1.4 | Raw scores as persistent default view (not buried behind Details toggle) | Marcus, Fatima | Should Have | Sprint 20 / Phase A |
| 2.1 | Frame-level annotations on video timeline (named markers at timestamps) | James | Should Have | Phase A |
| 2.6 | Video frame thumbnails embedded in PDF trust report | James, Amara | Should Have | Phase A |
| 3.3 | JPEG ghost artefact map overlay (localised visualisation of re-compression regions) | Fatima | Should Have | Phase A |
| 4.6 | Database filter/sort by EXIF anomaly, metadata completeness, trust score | Amara, Fatima | Should Have | Phase A |
| 2.5 | Encoder string interpretation (human-readable explanation of encoder type and implications) | James | Should Have | Phase A |
| 1.5 | C2PA provenance chain timeline (full chain of events, not just latest manifest) | Marcus, Amara | Should Have | Phase A |
| 1.6 | GPS coordinate map preview in EXIF panel (OpenStreetMap link or embedded preview) | Marcus, Fatima, James | Should Have | Phase A |
| 2.7 | Incremental analysis streaming (results populate as each mode completes) | James, Marcus | Should Have | Phase B |
| 2.2 | Audio waveform display with visual edit-point detection | James | Could Have | Phase B |
| 2.4 | Audio edit-point annotations on waveform timeline | James | Could Have | Phase B |
| 3.6 | Batch perceptual hash comparison within a case or session (near-duplicate matrix) | Fatima, Amara | Could Have | Phase B |
| 1.3 | Wire service archive integration (AP, Getty, Reuters perceptual hash lookup) | Marcus, James | Could Have | Phase B |
| 4.4 | "Verify and archive" mode (authenticated copy + preserved original + chain-of-custody link) | Amara | Could Have | Phase B |
| 4.7 | Cross-collection near-duplicate detection (perceptual hash distance matrix across full collection) | Amara | Could Have | Phase B |
| 3.2 | Opt-in external weather/event cross-reference (pre-filled URL redirect, no cloud call) | Fatima | Could Have | Phase B |
| 1.1-b | BYOK TinEye reverse image search (hash-based, no raw image upload) | Marcus, James | Should Have | Phase A |
| 2.3 | Reverse video search integration (InVID/WeVerify or equivalent) | James | Could Have | Phase B |

---

### Cross-Persona Gap Consensus

The following gaps are identified by three or more of the four personas. These represent the highest-confidence design signals — they should be treated as hard priorities regardless of technical complexity.

**Unanimous or near-unanimous (3–4 personas)**

- **Free-text analyst annotations in the trust report** (Marcus, Fatima, Amara, James): All four personas maintain parallel notes documents during analysis. The annotation is not part of the trust report. This is a unanimous workflow gap.
- **Raw scores as default or easily accessible view** (Marcus, Fatima, James): Three of four practitioners want the raw numerical scores accessible without additional navigation. The current traffic-light summary is appropriate for non-specialist audiences but not for professionals building evidence chains.
- **Methodology and model version in exported PDF** (Fatima, Amara, James): Reproducibility and legal admissibility both require knowing which exact versions of models and algorithms produced the scores. This is described in Phase A but should be treated as a v1.0 requirement for Professional and Team tiers.
- **GPS/location information as actionable data** (Marcus, Fatima, James): EXIF GPS coordinates are surfaced as raw numbers. Three personas want them rendered as something actionable — a map preview, an OpenStreetMap link, or at minimum a DMS-formatted copy-paste string for external lookup.

**Two-persona consensus**

- Perceptual hash comparison between two images (Marcus, Fatima)
- Batch VERIFY queue (Fatima, Amara)
- Bulk export for large collections (Fatima, Amara)
- Video frame thumbnails in PDF export (James, Amara)

---

### Design Tensions

Three unresolved tensions emerge across the four scenarios that require explicit design decisions:

**Tension 1: Non-destructive evidence handling vs. current PROTECT workflow**
The PROTECT tab is designed to modify files (C2PA signing, watermarking). Amara cannot use PROTECT on evidence without altering the originals. The "Verify and archive" pattern — which separates authenticated copies from originals — requires a different interaction model that does not currently exist. Resolving this requires either a separate VERIFY-to-archive export pathway, or a non-destructive mode flag in PROTECT that makes copying explicit rather than implicit.

**Tension 2: Individual precision vs. batch throughput**
Marcus and James need to examine one image or one video in depth, with full control over the analysis parameters and the ability to drill into specific signals. Amara and Fatima need to process hundreds of items with minimal per-item interaction time, accepting some reduction in depth in exchange for coverage. These are fundamentally different interaction models. The current VERIFY tab is optimised for individual depth. A batch processing mode requires a different primary workflow, not just a "run all" button.

**Tension 3: Self-contained trust report vs. external evidence integration**
The PDF trust report captures what Jura Trace can measure. All four practitioners need to integrate external evidence — reverse image search results, weather records, geographic matching, source corroboration — into their final report. Currently this integration is entirely manual, happening in external documents or in post-export PDF editing. A structured case file format — containing the Jura Trace report, analyst annotations, and external evidence references as a single exportable package — would close this gap. This is related to the ZIP case export function already in the codebase (`ui/src/lib/zip.ts`) but that function does not currently include analyst notes or external evidence references.

---

### Recommended Phase A Priorities (Post-v1.0, July–August 2026)

Based on the gap analysis, the following Phase A investments would unlock adoption across all four professional practitioner personas:

1. **Batch VERIFY queue with summary table** — unlocks Amara's entire workflow and substantially accelerates Fatima's. The single highest-leverage feature for institutional adoption.

2. **Free-text analyst annotations exported in trust report** — closes a unanimous workflow gap with modest development effort. The annotation model already exists conceptually in the analyst declaration modal.

3. **Image comparison mode (two-image side-by-side with perceptual hash distance and visual diff)** — directly enables the temporal misattribution investigation that is one of the most common fact-checking scenarios.

4. **Methodology and model version in exported PDF** — low development effort (add version metadata to the PDF generation pipeline), high evidentiary value, required for Professional and Team tier credibility.

5. **Input file SHA-256 hash at import** — essential for chain-of-custody workflows. Relatively straightforward to implement as a Rust operation at the point of file import, recorded in the audit log and surfaced in the trust report.

6. **JPEG ghost artefact map overlay** — elevates the existing JPEG ghost detector from a binary flag to a genuinely useful forensic tool for the misattribution and evidence authentication workflows.

7. **GPS coordinate map preview / OpenStreetMap link** — low development effort (render GPS coordinates as a clickable link in the EXIF panel), removes a context-switching step from three of the four practitioner workflows.

---

*Document prepared by the Jura Trace persona testing agent. Scenarios are representative of professional practitioner workflows. Tool capabilities described reflect Jura Trace v0.9.0-rc.3 feature set as documented in CLAUDE.md and PROJECT_SPEC.md.*
