# Data Protection Impact Assessment — Jura Trace Deployment

**Template version**: 1.0
**Prepared by**: Juralabs Community Interest Company
**Date**: 25 March 2026
**Jura Trace version**: v0.9.0-rc.1
**ICO DPIA guidance reference**: https://ico.org.uk/for-organisations/guide-to-data-protection/guide-to-the-general-data-protection-regulation-gdpr/data-protection-impact-assessments-dpias/

---

**How to use this template**

This document is approximately 80% pre-filled with information specific to Jura Trace. Sections marked **[FOR ORGANISATION TO COMPLETE]** require input from your Data Protection Officer (DPO) or nominated data protection lead before deployment can proceed.

Jura Trace is a local-first desktop application. All processing occurs on the device where the application is installed. No personal data is transmitted to Juralabs or any third party during core operation. This architecture significantly reduces — but does not eliminate — data protection risk. This DPIA helps your organisation document its assessment of residual risk.

This template follows the seven-step structure recommended by the UK Information Commissioner's Office (ICO).

---

## Step 1: Identify the Need for a DPIA

### 1.1 Processing activity

Content verification and protection using the Jura Trace desktop application. The application analyses digital media files (images, video, audio, PDF documents) to detect manipulation, verify authenticity, embed provenance credentials (C2PA), and apply invisible watermarks.

### 1.2 Is a DPIA legally required?

**Assessment: A DPIA is likely not mandatory under UK GDPR Article 35, but is recommended as best practice.**

A DPIA is required under Article 35(1) UK GDPR where processing is "likely to result in a high risk to the rights and freedoms of natural persons." The ICO's screening criteria (published list under Article 35(4)) and the European Data Protection Board's guidelines (WP 248 rev.01) identify the following triggers:

| DPIA Trigger | Applicable to Jura Trace? |
|---|---|
| Systematic and extensive profiling with significant effects | No. Jura Trace performs ad hoc file-by-file analysis at user discretion. No profiling of data subjects occurs. |
| Large-scale processing of special category data (Article 9) | Unlikely. Photographs of identifiable individuals are personal data but not special category data unless the processing is specifically designed to identify racial origin, health, biometric identity, etc. Jura Trace does not perform facial recognition or biometric identification. |
| Systematic monitoring of publicly accessible areas | No. Jura Trace analyses files selected by the operator. It does not monitor public spaces or scrape public content. |
| New technologies (ICO screening criterion) | Partially. AI-based forensic analysis of media content involves newer technologies. This factor alone warrants a proportionate assessment. |
| Evaluation or scoring of individuals | No. Trust scores relate to the authenticity of a media file, not to the behaviour or characteristics of any data subject. |
| Automated decision-making with legal or significant effects | No. Jura Trace produces advisory forensic reports. It does not make automated decisions affecting data subjects. |

**Conclusion**: Fewer than two screening criteria are met. A mandatory DPIA is therefore unlikely to be required. However, because the application processes media that may depict identifiable individuals, and because it employs AI-based analysis techniques, completing this DPIA as a voluntary best-practice exercise is strongly recommended. This approach is consistent with ICO guidance that organisations should "do a DPIA for any major new project involving the use of personal data" (ICO DPIA guidance, "When do we need to do a DPIA?").

### 1.3 [FOR ORGANISATION TO COMPLETE]

- [ ] Confirm whether your organisation's internal data protection policy requires a DPIA for this type of processing regardless of the legal threshold
- [ ] Record the decision-maker who authorised proceeding with this DPIA (name, role, date)

| Field | Value |
|---|---|
| Decision-maker name | |
| Decision-maker role | |
| Date authorised | |
| Rationale for conducting DPIA | |

---

## Step 2: Describe the Processing

### 2.1 Nature of processing

Jura Trace performs local forensic analysis of digital media files. "Local" means all computation occurs on the device where the application is installed. The application consists of three components, all running on the same machine:

1. **Tauri desktop application** (Rust core + SvelteKit user interface) — reads files selected by the user, performs C2PA verification, perceptual hashing, EXIF metadata extraction, watermark embedding/extraction, and stores analysis results in a local SQLite database.
2. **Python ML sidecar** (FastAPI, localhost port 8200) — performs AI-based forensic analysis including error level analysis (ELA), noise pattern analysis, copy-move detection, deepfake detection, and additional specialist detectors. Communicates only via the device's loopback interface (127.0.0.1). No network traffic leaves the device.
3. **Ollama LLM runtime** (optional, localhost port 11434) — provides local large language model inference for image description and claim verification. Entirely optional. When absent, the application operates without these features.

**Critical architectural point**: Original file content (image pixels, video frames, audio waveforms) is **not** stored in the database. Files are read into memory for analysis, results are computed, and only metadata, hashes, and analysis scores are persisted. The original file remains at its original location on the user's filesystem, unmodified (unless the user explicitly chooses to embed a watermark or C2PA signature, which modifies the file in place or creates a new output file).

### 2.2 Purpose of processing

- Verify the authenticity of digital media (detect manipulation, AI generation, or undisclosed editing)
- Protect original content by embedding provenance credentials (C2PA manifests) and invisible watermarks
- Generate forensic trust reports documenting the analysis methodology and findings
- Maintain an audit trail of verification activities for institutional record-keeping

### 2.3 Data subjects

Individuals who may appear in or be identifiable from the media files submitted for analysis. The application does not target specific data subjects; it analyses whatever files the operator selects.

Potential categories include:
- Individuals depicted in photographs (subjects of news images, archival photographs, insurance claim photographs, identification documents)
- Individuals whose voices appear in audio or video recordings
- Individuals identifiable from file metadata (photographer name in EXIF data, author fields in document metadata)

### 2.4 Categories of personal data processed

| Data Category | Source | Processing | Persisted? |
|---|---|---|---|
| Facial images of identifiable individuals | Photographs selected by operator | Analysed in-memory by forensic detectors. Not used for identification. | No (original pixels not stored) |
| Voice recordings | Audio/video files selected by operator | Transcribed locally via Whisper model (optional). Transcription text may contain personal data. | Transcription text stored in SQLite if transcription feature is used |
| Photographer/author identity | EXIF, XMP, IPTC metadata embedded in files | Extracted and displayed to operator | Summary metadata stored in SQLite |
| GPS coordinates | EXIF geolocation tags in photographs | Extracted as part of EXIF anomaly analysis | Stored as part of metadata summary in SQLite |
| Device identifiers | EXIF camera serial number, lens model, software version | Extracted as part of metadata analysis | Stored as part of metadata summary in SQLite |
| File paths on operator's device | User's local filesystem | Used to locate and read files | Stored in SQLite (asset records) |

**Special category data (Article 9 UK GDPR)**: Jura Trace does **not** perform facial recognition, biometric identification, or any processing specifically designed to reveal racial or ethnic origin, political opinions, religious beliefs, trade union membership, health data, sex life, or sexual orientation. However, if photographs submitted for analysis happen to depict individuals in contexts that reveal such information (e.g., a photograph taken at a political rally or a medical facility), the operator should be aware that such data passes through the application's in-memory analysis pipeline. It is not extracted, classified, or stored as special category data by the application.

### 2.5 Data flows

```
                                    LOCAL DEVICE BOUNDARY
  ┌─────────────────────────────────────────────────────────────────────────┐
  │                                                                         │
  │  [User selects file]                                                    │
  │         │                                                               │
  │         ▼                                                               │
  │  [Tauri App reads file into memory]                                     │
  │         │                                                               │
  │         ├──────────────────────────────┐                                │
  │         │                              │                                │
  │         ▼                              ▼                                │
  │  [Rust core engine]           [Python ML sidecar]                       │
  │  - C2PA verification           - ELA, noise, deepfake                   │
  │  - EXIF extraction             - Copy-move detection                    │
  │  - Perceptual hashing          - Video frame analysis                   │
  │  - Watermark extraction        - Audio transcription                    │
  │         │                      (localhost:8200 only)                     │
  │         │                              │                                │
  │         ▼                              │                                │
  │  [Local SQLite database] ◄─────────────┘                                │
  │  - Metadata summaries                                                   │
  │  - Analysis scores                                                      │
  │  - Audit trail (hash chain)                                             │
  │         │                                                               │
  │         ▼                                                               │
  │  [PDF trust report]  [ZIP case export]                                  │
  │  (saved to local filesystem)                                            │
  │                                                                         │
  └─────────────────────────────────────────────────────────────────────────┘

               No data crosses the device boundary during core operation.
```

**Exception — Enterprise tier optional cloud AI API**: Organisations using the Enterprise tier may optionally enable a third-party AI API for enhanced claim verification. When enabled:
- Only extracted text claims (not images, audio, or video content) are sent to the external API
- The organisation provides its own API key (BYOK model — Juralabs has no access to the key or the API responses)
- Each API call is recorded in the local audit log
- This feature requires explicit opt-in; it is disabled by default
- Recommended provider (Mistral AI) offers EU data residency guarantees

### 2.6 Data recipients

| Recipient | Data shared | Legal basis |
|---|---|---|
| Jura Labs CIC | None (no telemetry, no crash reports, no usage data) | N/A |
| Third-party AI API provider (Enterprise only, optional, disabled by default) | Extracted text claims only (no media content) | Organisation's legitimate interest; explicit operator consent at point of use |
| Recipients of exported PDF reports or ZIP case files | Analysis results, metadata summaries, trust scores, verdict | Determined by organisation's own sharing policies |

### 2.7 [FOR ORGANISATION TO COMPLETE]

- [ ] Describe your specific use case for Jura Trace (e.g., insurance claims verification, editorial content checking, archival authentication, academic integrity)
- [ ] List the categories of individuals whose media you expect to process
- [ ] Define your data retention period for the SQLite database and exported reports
- [ ] Identify who within your organisation will have access to the application, its database, and exported reports
- [ ] Confirm whether you intend to enable the optional Enterprise cloud AI API feature

| Field | Value |
|---|---|
| Specific use case | |
| Categories of data subjects | |
| Database retention period | |
| Report retention period | |
| Authorised users (roles) | |
| Cloud AI API enabled? | Yes / No |
| Cloud AI API provider (if yes) | |

---

## Step 3: Consultation

### 3.1 Vendor consultation

Jura Labs CIC provides this DPIA template, the accompanying Information Security Summary document, and the application's security audit report (available on request to Enterprise and Grant-Subsidised Access tier licensees). Juralabs' pre-release security penetration test (25 March 2026) identified 0 critical, 4 high, 5 medium, 6 low, and 4 informational findings. All high-severity findings were remediated on the date of identification. The full report is available under NDA.

No systemic data protection concerns have been identified by the vendor. The local-first architecture was selected specifically to minimise data protection risk.

### 3.2 Technical security measures confirmed by vendor

- All sidecar communication is restricted to the loopback interface (127.0.0.1); no network egress
- Sidecar API key authentication is enforced in production (auto-generated per installation)
- Content Security Policy (CSP) is pinned to `127.0.0.1:8200` (sidecar) and `127.0.0.1:11434` (Ollama) only
- All Tauri IPC commands that accept file paths perform null-byte checking and path canonicalisation
- Shell execution capabilities are scoped to the bundled sidecar binary only; no arbitrary command execution
- Audit trail uses SHA-256 hash chain for tamper evidence
- URL verification includes SSRF prevention (loopback and private network blocking)
- Base64 image data is converted to blob URLs to eliminate CSP `data:` scheme usage
- Query string parameters are redacted in all logging
- Temporary files created during sidecar analysis are cleaned up on all code paths, including exceptions
- Python dependencies are pinned to exact versions with `pip-audit` in CI

### 3.3 [FOR ORGANISATION TO COMPLETE]

- [ ] Record DPO consultation date and outcome
- [ ] Record any staff or team consultation undertaken
- [ ] Identify whether data subjects should be notified of processing (likely not required for ad hoc forensic analysis of media files, but your DPO should confirm based on your specific use case)
- [ ] If processing media depicting vulnerable individuals (children, victims of crime), confirm additional safeguards

| Field | Value |
|---|---|
| DPO name | |
| DPO consultation date | |
| DPO recommendation | Proceed / Proceed with conditions / Do not proceed |
| Conditions (if any) | |
| Staff consultation conducted? | Yes / No |
| Data subject notification required? | Yes / No |
| Vulnerable individuals safeguards | |

---

## Step 4: Assess Necessity and Proportionality

### 4.1 Lawful basis for processing

**Recommended lawful basis: Legitimate interest (Article 6(1)(f) UK GDPR)**

The three-part legitimate interest assessment:

| Element | Assessment |
|---|---|
| **Purpose test** — Is there a legitimate interest? | Yes. Verifying the authenticity of digital media serves legitimate organisational interests including: editorial accuracy, insurance fraud prevention, legal evidence integrity, academic integrity, cultural heritage protection, and compliance with emerging content authenticity regulations (EU AI Act, Article 50(2)). |
| **Necessity test** — Is the processing necessary for that purpose? | Yes. Forensic analysis of media files requires computational processing of the file content. There is no less intrusive way to assess whether an image has been manipulated or AI-generated. The application processes only the files the operator selects — there is no bulk collection, automated scanning, or persistent monitoring. |
| **Balancing test** — Do the individual's interests override? | Unlikely. The processing is: (a) limited to files already in the organisation's possession or control; (b) performed locally with no data leaving the device; (c) focused on the media content's technical characteristics rather than on identifying or profiling the data subjects depicted; (d) advisory only — it produces forensic indicators, not automated decisions affecting individuals. The impact on data subjects is minimal. |

**Alternative lawful bases** that may apply depending on your use case:
- **Legal obligation** (Article 6(1)(c)) — where content verification is required by regulation (e.g., EU AI Act transparency obligations from August 2026)
- **Public task** (Article 6(1)(e)) — for public authorities or cultural institutions performing statutory functions
- **Contract** (Article 6(1)(b)) — where content verification is part of a service agreement (e.g., insurance claims investigation)

### 4.2 Data minimisation

- The application processes only files explicitly selected by the operator
- Original file content is read into memory for analysis and then discarded; it is not stored in the database
- Only metadata, perceptual hashes, and analysis scores are persisted
- The operator controls which files are submitted and when
- No background scanning, indexing, or automated discovery of files occurs
- EXIF metadata extraction captures only the fields relevant to anomaly detection and provenance assessment

### 4.3 Purpose limitation

Analysis results are generated solely for content verification and protection purposes. The application does not repurpose data for marketing, profiling, behavioural analysis, or any secondary purpose. Exported PDF reports and ZIP case files contain only the analysis results relevant to the verification conducted.

### 4.4 Storage limitation

- The SQLite database is stored at a user-configurable location on the local filesystem
- The database can be deleted at any time by the operator (a single file)
- There is no minimum retention period imposed by the application
- Audit trail entries are retained according to the organisation's configured policy (default: 12 months on Community tier, 24 months on Professional and above)
- The application does not enforce any maximum retention period; this is the organisation's responsibility

### 4.5 Accuracy

Jura Trace produces **probabilistic forensic indicators**, not deterministic truth claims. The three-way verdict system (Authentic / Inconclusive / Synthetic) is specifically designed to avoid false certainty. The application's documentation, methodology panels, and PDF reports all include the following disclosures:

- Forensic analysis provides evidence of manipulation indicators, not proof of manipulation
- No single detector result should be treated as conclusive
- The Signal Agreement dashboard shows where detectors agree or disagree, encouraging multi-signal reasoning
- The Visual Inspection Checklist prompts human examination alongside automated analysis

This design mitigates the accuracy risk inherent in any AI-assisted analysis system.

### 4.6 [FOR ORGANISATION TO COMPLETE]

- [ ] Confirm the lawful basis for your specific use case
- [ ] If relying on legitimate interest, complete and retain a Legitimate Interest Assessment (LIA) as a separate document
- [ ] Document any additional purposes for which analysis results may be used (e.g., sharing with law enforcement, regulatory submission, publication)
- [ ] Define your data retention policy for the Jura Trace SQLite database
- [ ] Define your retention policy for exported PDF reports and ZIP case files

| Field | Value |
|---|---|
| Confirmed lawful basis | |
| LIA completed? (if legitimate interest) | Yes / No |
| LIA document reference | |
| Additional processing purposes | |
| SQLite database retention period | |
| PDF report retention period | |
| ZIP case export retention period | |

---

## Step 5: Identify and Assess Risks

### 5.1 Pre-assessed risks

The following risks have been identified and assessed by Jura Labs CIC based on the application's architecture and security posture.

| # | Risk | Likelihood | Severity | Risk Level | Mitigation (built-in) | Additional mitigation (organisation) |
|---|---|---|---|---|---|---|
| R1 | Unauthorised access to analysis results stored in the local SQLite database | Low | Medium | LOW | Database is a local file with no network exposure. Access is governed by OS-level file permissions. | Enable full-disk encryption (FileVault / BitLocker / LUKS). Restrict application access to authorised personnel via OS user accounts. |
| R2 | False positive: authentic content incorrectly flagged as manipulated, leading to adverse action against a data subject | Very Low | Medium | LOW | Three-way verdict system avoids binary determinations. Signal Agreement dashboard highlights detector disagreement. Methodology statement in every report. Current cross-validation false positive rate: 4.54% (AUC 0.9868 on 10,709 images, 5-fold stratified CV; GBM v4 trained 7 April 2026). | Establish internal policy that Jura Trace results are advisory only and must be reviewed by a qualified human before any action is taken. |
| R3 | False negative: manipulated content assessed as authentic, leading to reliance on fabricated media | Low-Medium | High | MEDIUM | Deep analysis mode runs the four regional detectors (Segmented ELA, Shadow Consistency, Colour Temperature, Splice Boundary) in addition to the core suite, plus 20 video frames vs 6 in Standard. Trained GBM v4 + UnivFD v9 ensemble classifier supplements heuristic scoring. 12 automatic forensic detectors plus 3 on-demand investigation tools provide defence in depth. | Use Deep mode for high-stakes analysis. Cross-reference with additional sources. Do not rely on any single tool for critical decisions. |
| R4 | Personal data in EXIF metadata (GPS coordinates, photographer identity, device serial numbers) inadvertently retained beyond its useful life | Medium | Low | LOW | EXIF data is stored as a summary in the SQLite database. The application does not extract or index EXIF data beyond what is relevant to anomaly detection. | Define and enforce a retention policy for the SQLite database. Periodically purge analysis records that are no longer required. |
| R5 | Transcription text (from audio/video) contains personal data (names, addresses spoken in recordings) | Medium | Medium | MEDIUM | Transcription is an optional feature requiring the Whisper model. It is disabled when the model is absent. Transcription text is stored locally only. | Review transcription text before including it in shared reports. Apply your organisation's data handling policy to transcription outputs. |
| R6 | Sidecar communication intercepted | Very Low | Low | VERY LOW | All sidecar traffic is restricted to the loopback interface (127.0.0.1). Traffic never leaves the device. API key authentication is enforced. | No additional mitigation required. |
| R7 | Exported PDF report or ZIP case file containing personal data is shared inappropriately | Medium | Medium | MEDIUM | Reports are saved to local filesystem only. The application does not transmit, upload, or email reports. | Apply your organisation's information classification and sharing policies to all exported reports. Mark reports containing personal data appropriately. |
| R8 | Optional Enterprise cloud AI API transmits data to external service | Low (disabled by default; text claims only, not media content) | Medium | LOW | BYOK model — organisation controls API keys. Only extracted text claims are sent, never images or media. Explicit opt-in required. All calls logged in audit trail. Mistral AI (recommended provider) offers EU data residency. | Only enable if organisational policy permits external API calls. Review the third-party provider's data processing terms. Consider a separate DPIA addendum for this processing activity if enabled. |
| R9 | Database file included in unencrypted device backup or cloud sync | Medium | Medium | MEDIUM | Database is a standard SQLite file. The application does not control backup behaviour. | Ensure the database storage location is within an encrypted volume. Exclude the database directory from unencrypted cloud sync services if your information security policy requires this. |

### 5.2 [FOR ORGANISATION TO COMPLETE]

- [ ] Review the pre-assessed risks above and confirm or adjust the likelihood and severity ratings for your context
- [ ] Add any organisation-specific risks (examples below)
- [ ] Confirm your organisation's risk appetite for each identified risk
- [ ] Sign off residual risk acceptance

**Potential additional risks to consider:**

- Processing media depicting children or vulnerable individuals
- Processing media subject to legal privilege or sub judice restrictions
- Processing media containing classified or official-sensitive information
- Multiple operators sharing a single OS user account (undermining access controls)
- Deploying on shared or multi-tenant infrastructure

| Additional Risk | Likelihood | Severity | Mitigation | Risk Owner |
|---|---|---|---|---|
| | | | | |
| | | | | |
| | | | | |

---

## Step 6: Identify Measures to Reduce Risk

### 6.1 Measures built into Jura Trace

| Measure | Description | Relevant Risk(s) |
|---|---|---|
| Local-first architecture | All processing on-device. No cloud calls for core functionality. No telemetry. | R1, R6, R8 |
| Original content not stored | Image/video/audio pixel data is processed in memory only. Not written to the database. | R1, R4 |
| Sidecar API key authentication | Auto-generated API key required for all sidecar requests in production mode | R6 |
| Content Security Policy | CSP pinned to localhost only (127.0.0.1:8200 and 127.0.0.1:11434) | R6 |
| Path canonicalisation | All Tauri IPC commands validate and canonicalise file paths. Null-byte injection and directory traversal attacks are blocked. | R1 |
| Shell capability scoping | Frontend can only execute the bundled sidecar binary, not arbitrary system commands | R1 |
| SHA-256 hash chain audit trail | Every verification action is recorded with a cryptographic hash linking it to the previous entry, providing tamper evidence | R1, R7 |
| Three-way verdict system | Authentic / Inconclusive / Synthetic avoids false binary determinations | R2, R3 |
| Probabilistic confidence scores | All results presented as confidence indicators, not certainty claims | R2, R3 |
| Graceful degradation | Application functions without optional components (sidecar, Ollama, FFmpeg, CLIP, Whisper). Features degrade rather than fail. | R5 |
| SSRF prevention | URL verification blocks loopback and private network addresses | R1 |
| Temporary file cleanup | Sidecar cleans up temporary files on all code paths including exceptions | R4 |
| Dependency pinning and audit | Python dependencies pinned to exact versions; `pip-audit` runs in CI | R1 |

### 6.2 [FOR ORGANISATION TO COMPLETE]

- [ ] Confirm full-disk encryption is enabled on all devices where Jura Trace is deployed

  | Platform | Encryption technology | Enabled? |
  |---|---|---|
  | macOS | FileVault | Yes / No |
  | Windows | BitLocker | Yes / No |
  | Linux | LUKS | Yes / No |

- [ ] Define your access control policy for Jura Trace (who is authorised to use it and under what circumstances)
- [ ] Define your incident response procedure for any data protection issue arising from Jura Trace use
- [ ] Confirm whether exported reports will be classified under your information classification scheme
- [ ] If enabling the optional cloud AI API (Enterprise tier), confirm your organisation has reviewed the third-party provider's data processing agreement

| Field | Value |
|---|---|
| Access control policy reference | |
| Incident response procedure reference | |
| Report classification level | |
| Cloud AI DPA reviewed? (if applicable) | Yes / No / N/A |

---

## Step 7: Sign Off and Record Outcomes

### 7.1 [FOR ORGANISATION TO COMPLETE]

| Field | Value |
|---|---|
| **DPO name** | |
| **DPO approval date** | |
| **DPO signature** | |
| | |
| **Risk owner name** | |
| **Risk owner role** | |
| **Risk owner approval date** | |
| **Risk owner signature** | |
| | |
| **Senior responsible officer** | |
| **SRO approval date** | |
| **SRO signature** | |

### 7.2 Decision

- [ ] **Proceed** — risks are acceptable; no additional conditions required
- [ ] **Proceed with conditions** — risks are acceptable subject to the conditions listed below
- [ ] **Do not proceed** — risks are not acceptable; deployment is not approved

**Conditions (if applicable):**

1.
2.
3.

### 7.3 Review schedule

This DPIA should be reviewed:
- [ ] Annually from the date of approval
- [ ] When Jura Trace releases a major version update (e.g., v1.0 to v2.0)
- [ ] When the organisation's use case changes materially
- [ ] When a data protection incident occurs involving Jura Trace
- [ ] When relevant legislation changes (note: EU AI Act high-risk obligations take effect August 2026)

| Review # | Date | Reviewer | Outcome | Changes made |
|---|---|---|---|---|
| 1 | | | | |
| 2 | | | | |
| 3 | | | | |

---

## Appendix A: Jura Trace Data Inventory

| Data Element | Stored in Database? | Storage Location | Encrypted at Rest? | Default Retention | Contains Personal Data? |
|---|---|---|---|---|---|
| Original image/video/audio content | No | In-memory during analysis only | N/A (not persisted) | Discarded after analysis completes | Potentially (photographs of people, voice recordings) |
| File metadata (EXIF, XMP, IPTC summary) | Yes | SQLite database | OS-level (FileVault / BitLocker / LUKS) | User-configurable | Yes (photographer name, GPS, device IDs) |
| Perceptual hashes (aHash, dHash, pHash) | Yes | SQLite database | OS-level | User-configurable | No (one-way hash; original cannot be reconstructed) |
| C2PA manifest data | Yes (if present in source file) | SQLite database | OS-level | User-configurable | Potentially (signer identity in C2PA claims) |
| Forensic analysis scores (per-detector) | Yes | SQLite database | OS-level | User-configurable | No |
| Aggregate trust score and verdict | Yes | SQLite database | OS-level | User-configurable | No |
| Audit trail entries | Yes (SHA-256 hash chain) | SQLite database | OS-level | 12 months (Community) / 24 months (Professional+) | No (records action type, timestamp, file hash — not file content) |
| Transcription text (optional, requires Whisper) | Yes (when feature is used) | SQLite database | OS-level | User-configurable | Potentially (names, addresses spoken in recordings) |
| Monitor URL watchlist | Yes | SQLite database | OS-level | User-configurable | Unlikely (URLs of published content) |
| Sidecar API key | No (environment variable) | Process memory | N/A | Session only (regenerated on each launch) | No |
| Exported PDF trust reports | User's choice | Local filesystem (user-selected path) | OS-level | Organisation's retention policy | Potentially (metadata summaries, verdicts referencing specific files) |
| Exported ZIP case files | User's choice | Local filesystem (user-selected path) | OS-level | Organisation's retention policy | Potentially (includes report content) |
| Watermark payload (embedded in output files) | Embedded in file | Within the watermarked file itself | N/A (part of the file) | Lifetime of the file | No (128-bit UUID only) |

---

## Appendix B: Jura Trace Tier Relevance

This DPIA is relevant to all tiers of Jura Trace. The core data processing described in this document is identical across tiers. The following tier-specific considerations apply:

| Tier | Additional Considerations |
|---|---|
| **Community** (free, non-commercial) | No additional data processing beyond what is described in this DPIA. |
| **Professional** (£199/year) | Report customisation adds organisation name, analyst name, and case reference to exported reports. These are entered by the operator and stored locally. Reverse image search (MONITOR Layer 2) sends a perceptual hash to TinEye or Google Vision via the operator's own API key — this is an external data transfer requiring separate consideration. |
| **Team** (£79/seat/month) | Shared SQLite database on network storage may require additional access control assessment. Multiple operators accessing the same database increases the surface for unauthorised access to analysis results. |
| **Enterprise** (from £6,000/year) | Optional cloud AI API (see Risk R8). Silent installer deployment via MDM/GPO should be covered by the organisation's existing software deployment DPIA or equivalent. Custom RAG knowledge base import does not involve personal data unless the organisation's corpus contains it. |

---

## Appendix C: Relevant Legislation and Guidance

| Instrument | Relevance |
|---|---|
| **UK GDPR** (retained EU law, as amended by Data Protection, Privacy and Electronic Communications (Amendments etc) (EU Exit) Regulations 2019) | Primary data protection framework for UK-based deployments |
| **Data Protection Act 2018** (UK) | Supplements UK GDPR; provides for exemptions and special processing |
| **EU GDPR** (Regulation (EU) 2016/679) | Applicable where the deploying organisation is established in the EU or processes data of EU residents |
| **EU AI Act** (Regulation (EU) 2024/1689) | Article 50(2) transparency obligations for AI-generated content take effect 2 August 2025. Article 6 high-risk classification for AI systems. Jura Trace is a tool that assists human operators in detecting AI-generated content; it is not itself a "high-risk AI system" under Annex III, but organisations should be aware of their own obligations under the Act. |
| **ICO DPIA guidance** | https://ico.org.uk/for-organisations/guide-to-data-protection/guide-to-the-general-data-protection-regulation-gdpr/data-protection-impact-assessments-dpias/ |
| **EDPB Guidelines on DPIAs** (WP 248 rev.01) | Criteria for determining when a DPIA is required; screening checklist |
| **ICO guidance on AI and data protection** | https://ico.org.uk/for-organisations/guide-to-data-protection/key-dp-themes/guidance-on-ai-and-data-protection/ |

---

## Appendix D: Version History

| Version | Date | Author | Changes |
|---|---|---|---|
| 1.0 | 25 March 2026 | Jura Labs CIC | Initial template |

---

## Appendix E: Contact Information

**Vendor**: Juralabs Community Interest Company
**Website**: https://juralabs.org
**Product**: Jura Trace v0.9.0-rc.1
**Licence**: PolyForm Noncommercial 1.0.0 (Community tier) / Commercial licence (paid tiers)

For questions about this DPIA template or the data processing described herein, contact Jura Labs CIC via the website above.

---

*This template provides information to assist your organisation's Data Protection Impact Assessment. It does not constitute legal advice. Your organisation's Data Protection Officer or qualified legal counsel should review and approve this assessment before deployment proceeds.*
