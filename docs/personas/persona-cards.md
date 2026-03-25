---
title: "Jura Trace — Persona Cards"
description: "Full persona set for Jura Trace and Jura Check. Covers all four existing personas (updated against the post-v1.0 roadmap) plus six new personas for the expanded professional markets."
date: 25 March 2026
status: Living document — update after each major product milestone
version: 2.0
supersedes: Persona references in FINANCIAL_ROADMAP.md and commercial-expansion-findings.md
---

# Jura Trace — Persona Cards

**Version:** 2.0
**Date:** 25 March 2026
**Prepared by:** Juralabs — persona-testing agent
**Audience:** Co-founders, investors, grant applications, UX research

This document contains ten persona cards covering the full user population for Jura Trace and the Jura Check consumer concept. Personas are grouped by tier.

---

## How to Read These Cards

Each card uses a consistent structure:

- **Snapshot** — age, role, location, organisation type, tier
- **Technical comfort** — rated 1 (minimal) to 5 (expert)
- **Workflow** — what they do before, during, and after using Jura Trace
- **Primary use case** — the job they specifically need Jura Trace to do
- **Pain points** — what they struggle with currently
- **Key features** — what matters most in their context
- **Adoption blockers** — what would prevent them adopting the product
- **Discovery path** — how they would find Jura Trace
- **Budget and procurement** — how they pay and who approves it
- **Abandonment triggers** — what would make them stop using it

---

## Part 1: Existing Personas (Updated)

These four personas were the original Jura Trace design targets. They are updated here to reflect what has been built (Sprints 1–16) and the post-v1.0 roadmap.

---

### Persona 1 — Dr. Sarah Chen

**Museum Curator / Head of Digital Collections**

| Field | Detail |
|---|---|
| Age | 52 |
| Location | Edinburgh, Scotland |
| Organisation | Mid-sized national museum (300–500 staff; digital collections team of 8) |
| Tier | Community (free) — non-commercial use under PolyForm NC |
| Technical comfort | 2 / 5 |

**Background.** Sarah has led the digital collections team for nine years. She holds a PhD in art history and a postgraduate certificate in digital heritage management. She is fluent in metadata standards — Dublin Core, IPTC, CIDOC-CRM, Spectrum — but finds command-line interfaces alienating and avoids them. She uses the museum's collection management system daily. When a tool requires a terminal window, she asks a colleague to help.

**Workflow.**
- Receives high-resolution digitisation output from scanning team (TIFF, JPEG, occasionally MP4 for 3D rotating views)
- Applies metadata via the CMS, including provenance notes and rights statements
- Publishes selected images to the museum website and partner portals (Europeana, Wikimedia)
- Periodically discovers their images appearing in AI training datasets, third-party products, or news stories without attribution
- Has no current systematic process for detecting or responding to unauthorised use

**Primary use case.** Batch C2PA signing of the museum's existing digitised catalogue (estimated 12,000 images across 6 collections). Embed provenance metadata. Detect when published images reappear without attribution or have been modified.

**Pain points.**
- Board is demanding action on AI scraping after a public incident involving collection images appearing in a generative AI output
- Budget for specialist tools is limited; any cost must go through a formal procurement process
- Her archivists (two of whom job-share and have low digital literacy) would be the day-to-day users, not Sarah herself
- Existing workflow requires images to be handled carefully to avoid overwriting museum metadata — a tool that strips or overwrites EXIF/IPTC data is a serious risk
- Batch processing of 12,000+ images is essential; one-at-a-time workflows are not viable
- WCAG compliance matters: her team includes staff with low vision who rely on high-contrast UI

**Key features.**
1. Batch C2PA signing with watched-folder automation (Enterprise — flagged as desired v2.0 feature)
2. MONITOR tab for tracking published image URLs
3. Protect pipeline that preserves, not overwrites, existing IPTC/XMP metadata
4. Clear, non-technical language throughout
5. Windows compatibility (the museum estate is Windows 11)

**Adoption blockers.**
- Any step requiring the command line — her staff cannot action it
- A tool that overwrites existing Dublin Core or IPTC metadata fields would be refused immediately
- Costs requiring board-level approval (>£500) without demonstrated ROI
- No audit trail — the museum is subject to Spectrum standards and needs records of what was signed, when, and by whom

**Discovery path.** Museum sector networks: Museums Association, Collections Trust, IIIF conference. Recommendation from a peer institution's digital manager. Google search for "C2PA museum collections" or "protect images from AI training UK."

**Budget and procurement.** On Community (free) tier. Any paid tier would require a procurement business case to the head of operations. Has access to small digital innovation budgets (£2,000–5,000/year) through the Collections Trust partnership.

**Abandonment triggers.** If the tool modifies existing metadata on first run. If a batch signing job corrupts a file. If the sidecar installation requires command-line steps that her team cannot complete.

**Update notes (v2.0).** Original persona age updated from 58 to 52 (correcting an internal inconsistency). Technical comfort corrected from "limited" to 2/5 to align with card format. Key feature priority now includes MONITOR Layer 1 as a v1.0 need following Session 10 findings. Batch watched-folder signing correctly identified as Enterprise / v2.0 scope.

---

### Persona 2 — Marcus Olsen

**Investigative Journalist / Senior Reporter**

| Field | Detail |
|---|---|
| Age | 34 |
| Location | London, England |
| Organisation | Mid-tier digital news outlet (50–150 staff) |
| Tier | Community (free) — non-commercial journalism use |
| Technical comfort | 4 / 5 |

**Background.** Marcus has been an investigative reporter for eight years, primarily covering technology and misinformation. He is comfortable with OSINT tools, reverse image search, ffprobe for video inspection, and occasional command-line work. He tests tools quickly and makes decisions on workflow fit within the first ten minutes. If a tool slows him down, he drops it — deadline pressure is always present.

**Workflow.**
- Receives images and video via encrypted channels (Signal, SecureDrop), from sources he cannot identify publicly
- Runs quick reverse image searches (TinEye, Google Images) and EXIF inspection manually before deciding whether to investigate further
- When something looks suspicious, runs ELA or noise analysis using a cobbled-together set of free tools
- Writes up verification notes for his editor, who is not technically literate
- Publishes with a verification note in the article; occasionally the verification process becomes part of the story

**Primary use case.** Fast, on-device verification of images and video received from sources. Needs a result he can explain to a non-technical editor. Cannot upload suspicious material to cloud services — source protection is a hard constraint.

**Pain points.**
- Current toolkit is fragmented: FotoForensics online (cannot use for sensitive material), ffprobe manually, Google reverse image search. No single tool.
- No confidence score he can cite to an editor. "I ran some tools and it looked fine" is not defensible.
- Verification process takes 45–90 minutes per image using current methods; he has 20-minute windows before publication decisions are made
- Colleagues at the outlet have less forensic skill; he cannot delegate verification to them with current tools

**Key features.**
1. Fast VERIFY pipeline — Standard mode result in under 60 seconds
2. Trust report PDF exportable to show an editor
3. EXIF anomaly detection with plain-language explanation of what was found
4. ELA and deepfake detection working offline
5. Signal agreement dashboard so he can understand which detectors fired and why

**Adoption blockers.**
- Sidecar startup is manual (uvicorn command) — would accept a bundled autostart, not a manual terminal step
- If the PDF trust report uses jargon his editor cannot parse
- If VERIFY requires uploading to any external service (hard line — source protection)
- If Standard mode takes more than 90 seconds on a typical news-format JPEG

**Discovery path.** Word of mouth from investigative journalism network (Bellingcat Slack, Journalism++ community, IJNet). Mentioned in an OSINT toolkit roundup. Blog post from WITNESS or First Draft.

**Budget and procurement.** On Community (free) tier. His outlet has no tool procurement process for individual reporters; he acquires tools himself. Would pay £199/year Professional if his outlet required court-ready reports — but this is not yet a standard expectation in his workflow.

**Abandonment triggers.** If verification takes longer than existing manual process. If the verdict is inconsistent with what his own inspection suggests and there is no explanation for the discrepancy. If the tool requires any cloud connection for core analysis.

**Update notes (v2.0).** Browser extension moved from "primary want" to "Phase B / post-v1.0 roadmap" per Session 10 findings. MONITOR one-shot query mode identified as primary need (not watchlist). Speed threshold clarified to 90 seconds for Standard mode — revised up from 60 seconds based on realistic sidecar benchmarks.

---

### Persona 3 — Fatima Al-Rashid

**Fact-Checker / Senior Verification Analyst**

| Field | Detail |
|---|---|
| Age | 28 |
| Location | Birmingham, England |
| Organisation | Independent verification organisation (15–30 staff, similar to Full Fact or Logically) |
| Tier | Community (free) for individual use; Team (Geode, £79/seat/month) if the organisation adopts it for the 5-person verification team |
| Technical comfort | 5 / 5 |

**Background.** Fatima has a degree in data science and joined the fact-checking organisation three years ago from a data journalism role. She is comfortable with Python, APIs, and forensic analysis tools. She wants to understand what is happening under the hood — black-box verdicts make her uncomfortable. She reads methodology documentation before deploying any tool.

**Workflow.**
- Receives tip-offs via a monitored inbox, social media alerts, and referrals from journalists
- Initial triage: reverse image search, metadata inspection, social media source tracing
- Deeper analysis: ELA, noise analysis, JPEG ghost for high-priority items
- Builds an evidence chain linking the original source, analysis findings, and contextual fact-checking
- Publishes debunk reports that must be citable and reproducible — a fact-check that cannot be reproduced is as bad as not doing it

**Primary use case.** Batch verification of 50–200 images per investigation. Exportable reports with methodology disclosure. Integration into existing editorial workflow (their CMS is Superdesk; they use a shared Google Drive for evidence files).

**Pain points.**
- Volume of content to check is overwhelming. Single-file tools are too slow.
- Existing tools give traffic-light results with no explanation. When she challenges a verdict, she cannot trace which detector produced it.
- No reproducibility: if she runs the same image through a tool twice and gets different results, she cannot cite it.
- False positives damage credibility — publishing a debunk based on a false AI-detection result is career-damaging.
- Team of 5 analysts all need access; shared login is not acceptable for audit purposes.

**Key features.**
1. Batch VERIFY pipeline with results exportable to CSV or JSON
2. Raw signal scores (not just traffic lights) — she wants to see what ELA returned, not just "flagged"
3. Methodology disclosure: which detector version produced which result
4. False positive rate data by content type — she needs to know what the tool's error rate is
5. Signal agreement dashboard to identify when detectors disagree
6. Reproducibility: same input produces same output (deterministic pipeline)

**Adoption blockers.**
- Any pipeline step that is stochastic without a fixed random seed
- Methodology documentation that does not disclose model training data, version, or thresholds
- A team licence that requires a single shared database on a network drive before the feature is stable (she knows this is architecturally non-trivial; she does not want alpha features)
- If the false positive rate on authentic editorial photographs exceeds 8% — her organisation's credibility depends on not flagging real news photos as fakes

**Discovery path.** Mentioned in an IFCN (International Fact-Checking Network) newsletter. Tool comparison published by First Draft / Information Futures Lab. GitHub repository discovered via a data journalism conference (NICAR, ISOJ).

**Budget and procurement.** Personal tool use is Community (free). Organisational adoption requires a budget sign-off from her head of research — roughly £5,000/year for 5 seats at Team pricing is within discretionary budget. Would require a methodology review document before sign-off.

**Abandonment triggers.** If the false positive rate on authentic editorial photographs cannot be confirmed below 8%. If methodology versioning is absent (she needs to pin results to a specific model version). If batch processing breaks silently on edge-case file formats (AVIF, WebP, heavily compressed social media crops) without clear error output.

**Update notes (v2.0).** Now correctly identified as Team-tier candidate for organisational use (closes the Community-to-Enterprise gap that was identified in Sessions 9 and 10). Raw scores requirement updated to reference the methodology versioning feature (PV-A4) as the specific capability that addresses her reproducibility need.

---

### Persona 4 — Tom Williams

**IT Infrastructure Manager**

| Field | Detail |
|---|---|
| Age | 41 |
| Location | Cardiff, Wales |
| Organisation | Regional council (500+ staff; archives and communications departments are primary users) |
| Tier | Enterprise (Bedrock, from £6,000/year) — requires silent installer, MDM templates, central config |
| Technical comfort | 5 / 5 (infrastructure); 2 / 5 (ML/forensic domain knowledge) |

**Background.** Tom manages the IT infrastructure for a regional council, including endpoints, software procurement, security policy, and service desk operations. He evaluates every new application against four questions: Does it meet the council's security policy? Can I deploy it without touching 15 machines individually? What happens when it breaks? Can I justify the cost to procurement? He is not interested in forensic analysis methodology; he is interested in operational risk.

**Workflow.**
- Receives a request from the archives team (who have seen Jura Trace or been referred to it) to evaluate deployment
- Reads the deployment documentation and system requirements
- Tests in a sandboxed VM (Windows 11, standard council build, limited user rights)
- Identifies any policy conflicts (outbound network calls, unsigned binaries, runtime dependencies)
- Submits a procurement business case to his service manager

**Primary use case.** Deploy Jura Trace across 15 workstations for the archives team. Ensure it can be managed via Group Policy or Intune. Ensure it meets the council's outbound network policy (no cloud telemetry). Manage updates centrally.

**Pain points.**
- The Python ML sidecar is unfamiliar. What is a FastAPI service? What port does it use? What happens if it crashes?
- FFmpeg and Ollama are runtime dependencies that require separate installation and raise security review questions
- The council has an 8GB RAM standard build; Ollama is a blocker on that estate
- Unsigned binaries on macOS and Windows generate SmartScreen and Gatekeeper warnings that escalate to his service desk
- "Local-first" sounds good but he needs to verify there are no outbound calls before signing off

**Key features.**
1. Silent MSI installer with ADMX/Group Policy templates
2. Central TOML configuration file for organisation-wide defaults
3. No outbound network calls (confirmed via network traffic audit)
4. Clear system requirements table (CPU, RAM, disk) for each optional component
5. SHA-256 checksums for all installer files (currently absent — P1 finding from Session 12)
6. Deployment guide written for IT professionals, not end users

**Adoption blockers.**
- Absence of SHA-256 hash verification for installer downloads (identified as P1 blocker in Session 12)
- Windows Defender / SmartScreen warning on first launch with no clear resolution path
- Any runtime that requires admin rights at runtime (not just installation)
- Ollama as a hard dependency (not clearly optional in current documentation)
- No central update mechanism — he cannot push updates to 15 machines manually

**Discovery path.** Referred by the archives team manager who found the tool. He evaluates it on technical merit; discovery is not his domain.

**Budget and procurement.** Enterprise tier requires a formal procurement submission. He is not the budget holder (that is the archives service manager) but he controls the technical approval. His recommendation determines whether procurement proceeds. A compliance documentation pack (DPIA template, IS summary) is needed for the submission.

**Abandonment triggers.** If the deployment documentation does not answer his security review questions within the first read. If a test deployment on the sandboxed VM produces unexpected outbound traffic. If a sidecar crash requires end-user intervention to resolve.

**Update notes (v2.0).** Role updated from "Council heritage officer (47, low tech)" to the correct specification: IT Infrastructure Manager (41, high tech infrastructure, low domain knowledge). Age and technical profile corrected to align with the established persona specification. Windows Roaming profile database issue (Issue 3 from UX issues log) noted as outstanding.

---

## Part 2: New Personas — Professional and Enterprise Markets

Six new personas covering the commercial markets identified in the strategic reframing.

---

### Persona 5 — Niamh Gallagher

**Property Solicitor**

| Field | Detail |
|---|---|
| Age | 38 |
| Location | Manchester, England |
| Organisation | 12-partner commercial law firm (property and litigation practice) |
| Tier | Professional (Stratum, £199/year) |
| Technical comfort | 3 / 5 |

**Background.** Niamh is a senior associate specialising in property and commercial litigation. She handles conveyancing disputes, boundary conflicts, and land fraud cases. In the past 18 months she has encountered three cases where digitally manipulated evidence — altered floor plans, fabricated survey photographs, doctored planning permission documents — was submitted by an opposing party. Her current recourse is to instruct a forensic document examiner as an expert witness, which costs £3,000–8,000 per engagement and takes four to six weeks.

**Workflow.**
- Receives digital evidence from clients or opposing parties (photographs, PDFs, scanned documents)
- Reviews for obvious inconsistencies; currently relies on visual inspection and experience
- When suspicious, instructs a forensic expert witness and waits
- Uses the expert's report in court proceedings or to challenge evidence at disclosure stage

**Primary use case.** First-pass authentication of digital evidence (property photographs, scanned documents, PDF plans) before deciding whether to instruct a full expert witness. Not replacing expert witnesses — reducing the frequency of unnecessary engagements. Also: protecting her own client communications by signing documents with C2PA credentials before sharing.

**Pain points.**
- Expert witness cost (£3,000–8,000 per engagement) is prohibitive for lower-value disputes
- Four-to-six week turnaround means evidence questions arise after deadlines have passed
- No tool currently gives her a defensible first-pass assessment she can record in her file notes
- Client-privileged content cannot go through any cloud service
- Her firm has IT security review for any new software installation

**Key features.**
1. VERIFY pipeline with a PDF report she can attach to a file note (methodology disclosure required)
2. Report customisation: her name, firm name, case reference, analyst declaration
3. EXIF anomaly detection for photographs
4. Metadata inspection for PDFs (creation date inconsistencies, software fingerprints)
5. Methodology versioning: the report must state which version of which detector produced the result
6. Local-first: no cloud upload of client-privileged documents

**Adoption blockers.**
- If the forensic report does not include methodology disclosure, it is not usable as a file note exhibit
- If the tool cannot handle PDFs (she deals primarily with scanned documents and PDF plans)
- If the false positive rate is high enough to flag genuine documents — instructing an expert based on a false positive wastes everyone's time
- IT security review may require documentation she cannot currently find

**Discovery path.** Law firm technology conference (ILTACON, Legal Geek London). Legal technology newsletter (Briefed, Artificial Lawyer). Word of mouth from a barrister's chambers that has piloted the tool.

**Budget and procurement.** £199/year is below her personal expense approval threshold at the firm. She can purchase without going through procurement. If she recommends it to colleagues, a firm-wide licence (Team or Enterprise) would require IT security review and a procurement business case.

**Abandonment triggers.** If the tool flags a genuinely authentic document and she uses that result to instruct an unnecessary expert. If the PDF trust report is challenged in court because the methodology statement is inadequate. If a client privacy incident occurs due to cloud upload.

---

### Persona 6 — Elena Vasquez

**Insurance Claims Investigator**

| Field | Detail |
|---|---|
| Age | 44 |
| Location | Bristol, England |
| Organisation | Mid-sized insurance company (general insurance, personal lines) — 1,200 staff, SIU (Special Investigations Unit) of 9 |
| Tier | Team (Geode, £79/seat/month) for the SIU team of 9 |
| Technical comfort | 3 / 5 |

**Background.** Elena has been a claims fraud investigator for fourteen years. Her unit handles approximately 400 referred cases per year — motor, property, and personal injury claims flagged by the claims scoring algorithm as potentially fraudulent. Around 60% of her caseload involves image or video evidence submitted by claimants. She has seen a sharp increase in digitally manipulated claim evidence in the past two years and attributes much of it to consumer-grade image editing tools becoming widely accessible.

**Workflow.**
- Receives case referrals with attached digital evidence (photos of vehicle damage, property damage, injury photographs, dashcam clips)
- Reviews evidence manually and via the company's existing fraud scoring system (does not include image forensics)
- For suspicious cases, currently sends images to an external forensic firm (3–7 day turnaround, £200–600 per case)
- Files a fraud suspicion report with the Insurance Fraud Bureau if the case meets the threshold
- Prepares evidence packages for prosecution support

**Primary use case.** Batch analysis of incoming claim photographs (typically 5–30 images per case) to triage which cases warrant full forensic investigation. Reduce reliance on external forensic firm for first-pass assessment. Generate standardised reports for prosecution support files.

**Pain points.**
- External forensic firm turnaround is too slow for the claims workflow (decision must be made within 5 working days)
- Cost per case is too high to apply forensic analysis to all suspicious cases — currently only ~15% of flagged cases get forensic review
- No standardised report format for prosecution files — each external firm uses a different template
- The 9-person SIU needs a shared workflow: she cannot be the only person who can run the analysis

**Key features.**
1. Batch VERIFY pipeline: 30 images processed in one job, results exportable
2. Insurance sector-specific report template (sector-specific templates are a Team tier feature, PV-B2)
3. Shared asset database: all 9 SIU investigators access the same case database
4. Copy-move detection and JPEG ghost analysis — specific to the manipulation techniques she encounters (cloning undamaged areas over damaged areas in property claims)
5. Comparative analysis mode: "Is this the stock photograph that was edited?" (PV-B5, post-v1.0)
6. API integration: ideally plugging analysis into the existing claims management system (this is the long-term goal; requires PV-A2)

**Adoption blockers.**
- False positive rate above 5% is unacceptable: falsely accusing a genuine claimant of fraud is a compliance risk and a reputational risk
- If the batch processing breaks on compressed mobile phone photographs (the most common format in her caseload)
- If the shared database is not available at Team tier v1.0 — the team cannot use separate databases and then manually merge findings
- If the insurance report template is generic rather than structured around the specific evidence categories her prosecution files require

**Discovery path.** Insurance Fraud Bureau network. Chartered Insurance Institute conference. Recommendation from a claims director at another insurer who piloted the tool. Trade press (Insurance Post, Post Magazine).

**Budget and procurement.** Team licence for 9 seats at £79/seat/month = £711/month (£8,532/year). Within the SIU's annual fraud prevention budget (typically £15,000–30,000 for external tools). Requires head of SIU approval and IT security review. Procurement process is 6–12 weeks.

**Abandonment triggers.** If the tool flags genuine claim photographs as manipulated and this results in a wrongful fraud allegation. If the batch processing workflow is unreliable (partial results, silent failures). If the API integration (her long-term need) does not arrive on a credible roadmap.

---

### Persona 7 — James Okafor

**BBC Verify Desk Editor**

| Field | Detail |
|---|---|
| Age | 41 |
| Location | London, England |
| Organisation | BBC Verify (editorial verification unit within the BBC) |
| Tier | Team (Geode, £79/seat/month) — team of 8 analysts plus himself |
| Technical comfort | 3 / 5 (editorial background; manages technical analysts) |

**Background.** James joined BBC Verify at its launch and now leads a team of eight verification analysts and producers. His team verifies footage, photographs, documents, and social media posts for BBC output. The team uses a range of open-source and commercial tools: InVID/WeVerify, CrowdTangle (sunsetted), TinEye, Bellingcat's toolkit. He is not a technical expert but understands verification methodology well enough to evaluate tools and question results. His primary concerns are workflow efficiency, editorial defensibility, and protecting sources.

**Workflow.**
- Content arrives from BBC journalists, stringers, the newsgathering desk, and public tip-offs
- His analysts run verification checks against multiple tools simultaneously
- James reviews findings and makes the editorial call: publish, delay, or kill
- When a verification finding informs the story, it is documented and disclosed in the article
- Major stories require verification documentation to be retained for potential complaints or legal challenge

**Primary use case.** A unified verification tool that replaces or integrates with several fragmented tools currently used by his team. Specifically: replacing the manual combination of FotoForensics (cloud-based, source risk) + EXIF viewers + manual reverse image search with a single offline pipeline. The team processes 20–50 items per day on busy news days.

**Pain points.**
- Fragmented toolkit means no consistent methodology across the team — two analysts can run the same image through different tools and reach different conclusions
- FotoForensics and similar online ELA tools cannot be used for sensitive source material (potential security risk)
- No shared case database: findings are in individual analysts' files, not accessible to the full team
- Methodology documentation is weak — in a Leveson-era complaint, he needs to show exactly what analysis was run on a specific piece of content
- Onboarding new analysts takes two weeks just to set up the current fragmented toolkit

**Key features.**
1. Full VERIFY pipeline offline — ELA, EXIF, deepfake, watermark detection
2. Sector-specific newsroom verification report template (Team tier, PV-B2)
3. Shared asset database: all 8 analysts plus James working from one evidence store
4. Methodology disclosure in every exported report
5. Fast Standard mode: busy news days require results in under 90 seconds
6. Signal agreement dashboard: James wants his analysts to understand why two detectors disagree, not just accept a verdict

**Adoption blockers.**
- If the tool requires manual sidecar startup (uvicorn command) for each analyst's machine — not viable in a newsroom environment where machines are hot-swapped
- If the shared database (multi-seat SQLite) is not stable at launch — the team cannot operate from separate databases
- If the false positive rate flags genuine news agency photographs (Reuters, AFP, AP) as manipulated — editorial credibility damage
- BBC IT security requires all desktop software to pass a security review; unsigned binaries would not pass

**Discovery path.** Direct outreach from Juralabs (BBC Verify is explicitly named in the market outreach plan). Conference presentation at Global Investigative Journalism Conference or IJOC. WITNESS network referral. EMIF-funded tools evaluation.

**Budget and procurement.** Team of 9 (8 analysts + James) at £79/seat/month = £711/month (£8,532/year). Within the BBC Verify editorial tools budget. Procurement requires IT security review and editorial approval from the executive editor. Annual cycle — purchasing decision made in Q4 for following year. A six-month pilot at no cost or reduced cost would be required before budget commitment.

**Abandonment triggers.** If the sidecar requires manual restart after each machine reboot. If a high-profile editorial error occurs that is attributed to a Jura Trace false positive. If the shared database introduces file locking issues that disrupt the team workflow.

---

### Persona 8 — Amara Diallo

**Human Rights Documentation Coordinator**

| Field | Detail |
|---|---|
| Age | 33 |
| Location | London, England (field deployment in West Africa and the Middle East 3–4 months/year) |
| Organisation | International human rights NGO (200–400 staff globally; documentation unit of 12) |
| Tier | Grant-Subsidised Access (Enterprise-equivalent, free by application) |
| Technical comfort | 3 / 5 |

**Background.** Amara coordinates digital evidence collection across the NGO's field offices. Her team documents alleged human rights violations — extrajudicial killings, forced displacement, detention conditions — using photographs, video, and witness testimony. This evidence is used in UN submissions, ICC filings, and advocacy reports. Increasingly, evidence submitted by field researchers is challenged as AI-generated or manipulated by the governments and actors they are investigating. The "liar's dividend" — dismissing genuine evidence as fake — is a growing legal defence tactic.

**Workflow.**
- Field researchers submit evidence via secure channels (Signal, ProtonMail, encrypted USB)
- Amara's London team receives and catalogues evidence
- Content is verified before being used in any formal submission
- Evidence packages are prepared for legal proceedings, including chain-of-custody documentation
- Reports are submitted to UN Special Rapporteurs, ICC, and partner organisations

**Primary use case.** Authenticate field-collected evidence before use in formal submissions. Prove that genuine footage has not been manipulated. Generate chain-of-custody documentation that meets Berkeley Protocol standards. C2PA signing at or close to point of capture to establish provenance.

**Pain points.**
- The "liar's dividend": governments challenged two of her team's submissions in 2025 by claiming footage was AI-generated. They had no technical rebuttal capability.
- Evidence cannot be sent through cloud services — the organisations they investigate have surveillance capabilities and monitor major cloud providers
- Field researchers are not technically trained; any tool used in the field must be extremely simple
- Evidence packages must meet specific international legal standards (Berkeley Protocol); generic forensic reports are not sufficient
- Chain-of-custody documentation must be tamper-evident

**Key features.**
1. C2PA signing at point of capture or immediately after receipt (provenance from the first moment)
2. VERIFY pipeline with chain-of-custody audit trail
3. Batch processing: field teams submit evidence in batches, sometimes 200–500 items per incident
4. Human rights documentation report template (Berkeley Protocol-aligned) (Team/Enterprise tier, PV-B2)
5. Completely offline operation: no outbound calls, no telemetry, works in field environments with no internet connectivity
6. Tamper-evident audit log (the SHA-256 hash chain in Sprint 15 addresses this)

**Adoption blockers.**
- Any cloud dependency — absolute disqualifier in their security model
- Field deployment complexity: if installation requires command-line steps or Python setup, field researchers cannot manage it
- If the Berkeley Protocol alignment is not documented — her legal team would not accept a report without that reference
- The shared database (Team/Enterprise) must work on field laptops before being connected to a central store

**Discovery path.** WITNESS network and WITNESS Deepfake Rapid Response Force. Berkeley Protocol working group. OSJI (Open Society Justice Initiative). Mnemonic or Syrian Archive referral. Direct outreach from Juralabs (named as a target organisation in the market outreach plan).

**Budget and procurement.** Grant-Subsidised Access application. Her organisation qualifies on public benefit criteria. Funding comes from foundation grants (Open Society, Ford Foundation, Luminate). No commercial budget for tools. The grant-subsidised programme is the only viable path.

**Abandonment triggers.** If the audit log is not tamper-evident (a prosecution can challenge it). If the report format cannot be adapted to Berkeley Protocol requirements. If any outbound network call is discovered.

---

### Persona 9 — Richard Ashworth

**Corporate Communications Director**

| Field | Detail |
|---|---|
| Age | 49 |
| Location | London, England |
| Organisation | FTSE 250 listed company (financial services sector, 8,000 employees) |
| Tier | Enterprise (Bedrock, from £6,000/year) |
| Technical comfort | 2 / 5 |

**Background.** Richard has been head of communications for eight years. He manages a team of 14, covering media relations, investor relations, crisis communications, and internal communications. In 2024, a fabricated video purporting to show the company's CEO making damaging statements circulated on social media for four hours before it could be identified as a deepfake. The reputational and share price impact was significant. Since then, his CISO (Chief Information Security Officer) has asked him to put a verification capability in place before the next incident.

**Workflow.**
- Monitors media and social media for coverage of the company and its executives
- When suspicious content appears, currently contacts the corporate security team and waits
- Corporate security uses an external vendor (Reality Defender API, cloud-based) for deepfake detection — Richard considers this too slow for crisis response
- Needs to verify content within minutes, not hours, during a crisis

**Primary use case.** Rapid verification of deepfake video and audio content purporting to show company executives. Protecting legitimate corporate communications by signing them with C2PA credentials before distribution. Generating a forensic report within 10 minutes of discovering suspicious content.

**Pain points.**
- Current external vendor process takes 2–4 hours for a verified result — too slow during a live media crisis
- The forensic report from the external vendor uses technical language his team cannot interpret without the CISO present
- Client-confidential corporate content (pre-announcement communications, M&A materials) cannot go through a cloud service
- He has no way to prove that his company's own published video statements are authentic — the inverse of the deepfake problem
- His team does not have forensic skills; the tool must produce a clear verdict, not a signal dashboard

**Key features.**
1. Video deepfake detection with a clear three-way verdict (Authentic / Manipulated / Inconclusive)
2. Sub-10-minute analysis on a standard corporate laptop (video deepfake analysis in Standard mode ~12 seconds per frame × 6 frames = ~72 seconds plus overhead — this is feasible)
3. C2PA signing of legitimate corporate video and audio statements before distribution
4. Plain-language trust report suitable for sharing with the board and media counsel, not just the CISO
5. Local-first processing: sensitive pre-announcement content never leaves the device
6. White-label rights: his CISO wants to embed this capability in the corporate security dashboard under the company brand (Enterprise feature, v2.0 scope)

**Adoption blockers.**
- If the deepfake detection verdict is not clearly expressed — his team will misinterpret a nuanced technical result
- If the tool takes more than 15 minutes to produce a result during a live crisis (the current external vendor benchmark he is trying to beat)
- If the report cannot be shared with non-technical board members without the CISO present to interpret it
- Enterprise procurement process: he needs a DPIA, IS summary, and compliance documentation pack before his CISO will approve
- IT security will not accept an unsigned binary — code signing certificates are required at Enterprise

**Discovery path.** CISO peer network. Gartner Magic Quadrant research (Jura Trace is not yet on Gartner but a mention in an analyst note or competitive review would be how Richard discovers it). Recommendation from corporate security conference (Black Hat, RSA). Direct outreach via Juralabs contact with the corporate communications industry press.

**Budget and procurement.** Enterprise budget — cost is not the primary concern. His CISO controls the security tooling budget (£200,000–500,000/year for the security operations function). A Jura Trace Enterprise contract at £6,000–15,000/year is trivial against that budget. The blocker is procurement process, not cost. Full DPIA, vendor risk assessment, and IS review are required. Procurement cycle is 3–6 months.

**Abandonment triggers.** If a genuine corporate video is flagged as a deepfake during a critical communications moment. If the deepfake verdict is "Inconclusive" on a genuine fabricated video and the company suffers reputational damage as a result. If the compliance documentation is insufficient for the IS review.

---

## Part 3: Jura Check Persona

One consumer persona for the Jura Check product concept (decision gate: June 2026). This persona is exploratory — it informs the Phase 0 research and any eventual Jura Check design, not Jura Trace product decisions.

---

### Persona 10 — David Okonkwo

**Concerned Parent**

| Field | Detail |
|---|---|
| Age | 42 |
| Location | Leeds, England |
| Organisation | N/A — consumer |
| Product | Jura Check (not Jura Trace — separate product, decision gate June 2026) |
| Tier | Consumer freemium (Free / Parent tier, £3.99/month — Jura Check pricing, separate from Jura Trace tiers) |
| Technical comfort | 3 / 5 |

**Background.** David is a secondary school PE teacher and father of two: a 14-year-old daughter (Zara) and a 16-year-old son (Kwame). He is comfortable with technology in general — uses his phone confidently, manages family cloud storage, has a basic understanding of social media — but has no forensic or technical background. He became concerned about influencer content after Zara spent £85 on supplements recommended by a wellness influencer who, David later discovered, was being paid by the manufacturer without disclosing it.

**Workflow.**
- Occasionally browses the influencers his daughter follows (Instagram, TikTok, YouTube)
- Has no systematic process for evaluating them — relies on intuition and occasional Google searches
- When he finds something concerning, has a conversation with Zara about it — but without evidence or data, the conversation goes nowhere
- Would like a tool that gives him specific, factual talking points: "This influencer didn't disclose this was a paid promotion" rather than "I don't like this person"

**Primary use case.** Evaluate the credibility and transparency of specific influencers his daughter follows. Fact-check specific product claims before his daughter spends money on them. Start evidence-based conversations rather than blanket "I don't trust social media" arguments.

**Pain points.**
- He has no way to distinguish a genuine recommendation from a paid promotion
- Wellness and supplement claims are not obviously false but are often unsupported
- His daughter does not want him monitoring everything she does; he needs a light-touch approach, not surveillance
- Tools that treat parents as incompetent annoy him — he wants facts, not scare tactics
- Cannot afford to subscribe to a £99/month B2B tool; needs a consumer price point

**Key features (Jura Check — not Jura Trace).**
1. Plain-language trust report: "This influencer disclosed 3 of 7 paid partnerships in the last 30 days"
2. Specific product claim verification: "This supplement claim has no peer-reviewed evidence"
3. AI-generated content flags: "This image was likely AI-generated or heavily edited"
4. ASA/FTC compliance check: "This post promotes a product without clear sponsorship disclosure"
5. Conversation-starter format: a report he can share with Zara as a starting point for discussion, not a parental control tool

**Adoption blockers.**
- If it feels like surveillance software — Zara would refuse to engage if he approaches it as monitoring
- If the trust report is jargon-heavy — he needs plain English, not a forensic methodology statement
- If it gives false confidence: a "clean" report for an influencer who is actually problematic would be worse than no tool
- Price above £5/month — this is a discretionary household expense

**Discovery path.** UK government media literacy campaign ("You Won't Know Until You Ask"). NSPCC parent resources. A recommendation from another parent at his school. A news article about children and influencer marketing. App store search for "influencer fact check."

**Budget and procurement.** Consumer: no procurement process. Would pay up to £3.99/month without deliberation. Would pay £39/year (annual Parent tier) if the monthly trial was positive. Cost is not the primary concern at consumer price points — ease of use is.

**Abandonment triggers.** If the first check he runs gives a result that contradicts his own knowledge of the influencer (calibration failure). If setting up an account requires more than three steps. If his daughter finds out he is using the tool and considers it an invasion of privacy — he would stop rather than damage their relationship.

---

## Appendix A: Tier Assignment Summary

| Persona | Name | Role | Tier | Price |
|---|---|---|---|---|
| 1 | Dr. Sarah Chen | Museum Curator | Community | Free |
| 2 | Marcus Olsen | Investigative Journalist | Community | Free |
| 3 | Fatima Al-Rashid | Fact-Checker | Community / Team | Free / £79/seat/month |
| 4 | Tom Williams | IT Manager | Enterprise | From £6,000/year |
| 5 | Niamh Gallagher | Property Solicitor | Professional | £199/year |
| 6 | Elena Vasquez | Insurance Claims Investigator | Team | £79/seat/month |
| 7 | James Okafor | BBC Verify Desk Editor | Team | £79/seat/month |
| 8 | Amara Diallo | Human Rights Coordinator | Grant-Subsidised | Free (application) |
| 9 | Richard Ashworth | Corporate Comms Director | Enterprise | From £6,000/year |
| 10 | David Okonkwo | Concerned Parent (Jura Check) | Consumer freemium | £3.99/month |

---

## Appendix B: Key Feature Dependencies by Persona

This table maps the post-v1.0 backlog items (from `docs/strategic-pivot-assessment.md`) to the personas that need them.

| Backlog Item | ID | Niamh | Elena | James | Amara | Richard |
|---|---|---|---|---|---|---|
| FP rate below 5% | PV-A1 | Critical | Critical | High | High | Critical |
| API wrapper (port 8300) | PV-A2 | — | High | — | — | — |
| Report customisation (branding, case ref) | PV-A3 | Critical | High | High | — | High |
| Methodology versioning | PV-A4 | Critical | High | High | High | — |
| URL-based analysis | PV-B1 | — | — | High | — | — |
| Sector-specific report templates | PV-B2 | High | Critical | Critical | Critical | High |
| Browser extension | PV-B3 | — | — | Could | — | — |
| EU AI Act compliance dashboard | PV-B4 | Could | High | Could | — | Could |
| Comparative analysis mode | PV-B5 | High | Critical | Could | — | — |

---

## Appendix C: Discovery Channels Summary

| Channel | Personas Reached |
|---|---|
| Museum sector networks (Collections Trust, IIIF) | Sarah |
| Investigative journalism network (Bellingcat, Journalism++) | Marcus |
| IFCN / First Draft / data journalism conferences | Fatima |
| Law firm technology conferences (Legal Geek, ILTACON) | Niamh |
| Insurance Fraud Bureau / Chartered Insurance Institute | Elena |
| BBC Verify direct outreach / WITNESS network | James |
| WITNESS / Berkeley Protocol working group / OSJI | Amara |
| CISO peer network / security conferences | Richard |
| Parental safety organisations / UK government media literacy campaign | David |

---

*Version 2.0 — 25 March 2026. Next review: after v1.0 ships (June 2026) and after Phase 0 Jura Check parent interviews complete (May–June 2026).*
