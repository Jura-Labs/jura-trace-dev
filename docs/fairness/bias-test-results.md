---
title: "Demographic Bias Test Results — Jura Trace UnivFD v9"
sprint: "Sprint 29 — Fairness Foundation"
pillar: "TRIED Pillar 4 (Fair)"
generated: "2026-04-12T09:48:04+00:00"
test_result: "PASS"
status: "audit output — do not modify manually"
---

# Demographic Bias Test Results

**Generated:** 2026-04-12T09:48:04+00:00
**Sprint:** 29 — Fairness Foundation
**TRIED Pillar:** 4 (Fair)
**Model:** UnivFD v9 (AUC-ROC 0.9933, overall FP 4.12%, AI recall 95.70%)
**Test split:** `univfd_v9_split.json` (4334 images)
**Verdict: PASS**

---

## 1. Methodology

### Overview

This test uses CLIP ViT-B/32 cosine similarity to assign held-out test images to
demographic-proxy groups, then computes the UnivFD v9 model's FP rate and AI recall
within each group. The purpose is to detect gross disparities in model performance,
not to provide a demographic ground-truth audit.

### CRITICAL LIMITATIONS

**CLIP text-image similarity is NOT a demographic classifier.** It measures how
well a natural-language description matches an image's overall visual content, but:

1. Similarity scores are continuous and noisy. An image may score weakly on
   "a photograph of a dark-skinned person" because the person is small in the
   frame, in shadow, wearing distinctive clothing, or because the background
   dominates — not because the person is light-skinned.

2. Prompt-based grouping is not exclusive. Images near the threshold boundary
   may be assigned to multiple or no groups. The groups are fuzzy approximations.

3. CLIP ViT-B/32 itself has documented biases (trained on LAION-2B, English-
   language-dominant). Its similarity scores may be systematically less accurate
   for images from underrepresented geographic regions or cultural contexts —
   exactly the populations where fairness matters most.

4. **Passing this test is not a fairness certificate.** It is a canary: if the
   model had catastrophic disparities, they would likely show up here. The absence
   of flagged disparities does not exclude more subtle biases that would require
   human annotation to detect.

5. A proper demographic fairness audit requires human-annotated ground truth.
   This script is a computationally tractable interim measure, explicitly interim.

### Proxy Groups

| Group ID | Text Prompt | Purpose |
|---|---|---|
| `dark_skin` | "a photograph of a dark-skinned person" | Detect FP/recall disparity on darker-skinned subjects |
| `light_skin` | "a photograph of a light-skinned person" | Baseline comparison group |
| `no_people` | "a photograph with no people, showing scenery, objects, or animals" | Sanity check: non-person content should not be affected by demographic bias |
| `africa_south_asia` | "a photograph taken in Africa or South Asia" | Geographic proxy for Global Majority regions |
| `europe_north_america` | "a photograph taken in Europe or North America" | Geographic proxy for over-represented training regions |

Images are assigned to a group if their CLIP softmax probability for that prompt
exceeds the uniform baseline (1/n_groups = 20.0%), plus the argmax group is always included (non-exclusive — an image may belong to multiple groups).

### Assertion Thresholds

| Metric | Threshold | Rationale |
|---|---|---|
| FP rate ceiling | 2.0× overall FP rate (= 8.2%) | 2× is a common algorithmic fairness ceiling; chosen to flag large disparities without penalising small samples |
| AI recall floor | 85% | 15pp below overall recall (95.70%); flags substantial degradation |

---

## 2. Results

| Proxy Group | N Images | Auth Images | AI Images | FP Rate | AI Recall | FP Flag | Recall Flag |
|---|---|---|---|---|---|---|---|
| `dark_skin` | 815 | 308 | 507 | 7.8% | 97.8% | ok | ok |
| `light_skin` | 1942 | 980 | 962 | 5.0% | 97.1% | ok | ok |
| `no_people` | 2143 | 1201 | 942 | 3.1% | 95.2% | ok | ok |
| `africa_south_asia` | 1836 | 1129 | 707 | 3.5% | 95.0% | ok | ok |
| `europe_north_america` | 3765 | 2136 | 1629 | 3.5% | 95.4% | ok | ok |
| **Overall** | **4334** | **2379** | **1955** | **4.1%** | **95.7%** | — | — |

Overall FP rate: **4.1%** | Overall AI recall: **95.7%**

No assertions failed. All proxy groups are within the acceptable range (FP rate <= 8.2% and AI recall >= 85%). This does NOT mean the model is demographically fair — it means no gross disparity was detected by the CLIP-proxy method.


---

## 3. Per-Group Detail

### `dark_skin`

- Images in group: 815
- Authentic: 308 | AI: 507
- FP rate: 7.8% (24 FPs from 308 authentic)
- AI recall: 97.8% (496 TPs from 507 AI images)

### `light_skin`

- Images in group: 1942
- Authentic: 980 | AI: 962
- FP rate: 5.0% (49 FPs from 980 authentic)
- AI recall: 97.1% (934 TPs from 962 AI images)

### `no_people`

- Images in group: 2143
- Authentic: 1201 | AI: 942
- FP rate: 3.1% (37 FPs from 1201 authentic)
- AI recall: 95.2% (897 TPs from 942 AI images)

### `africa_south_asia`

- Images in group: 1836
- Authentic: 1129 | AI: 707
- FP rate: 3.5% (40 FPs from 1129 authentic)
- AI recall: 95.0% (672 TPs from 707 AI images)

### `europe_north_america`

- Images in group: 3765
- Authentic: 2136 | AI: 1629
- FP rate: 3.5% (75 FPs from 2136 authentic)
- AI recall: 95.4% (1554 TPs from 1629 AI images)

---

## 4. Recommendations

No assertion failures detected. Recommended actions for next quarterly cycle:

1. Expand the authentic corpus with Global Majority photography (see `corpus-demographic-profile.md` Section 7) to reduce geographic concentration risk before it manifests as a metric disparity.
2. Commission human annotation of at least 500 authentic images with skin-tone and geographic labels to replace CLIP-proxy grouping with ground-truth demographic grouping.
3. Add new generator families (Leonardo AI, HiDream, Midjourney v7, Flux Pro) to the AI corpus as they become available, and re-run this test to ensure recall is maintained across demographic proxy groups.

---

## 5. EMIF / TRIED Compliance Status

| Requirement | Status | Notes |
|---|---|---|
| Bias test methodology documented | Done | Section 1 |
| Per-proxy-group FP rate and recall | Done | Section 2–3 |
| Assertion thresholds defined | Done | FP ≤ 2.0× overall; recall ≥ 85% |
| Quarterly retrainability | Done | Script is parameterised; re-run after each retrain |
| Limitations explicitly stated | Done | Section 1 — CRITICAL LIMITATIONS |
| Human annotation follow-up planned | Recommended | Pending resource allocation |

**EMIF Pillar 4 citation**: This document, together with `corpus-demographic-profile.md`,
provides the evidence base for citing TRIED Pillar 4 (Fair) in the EMIF concept note.
The citation must be accompanied by the limitation caveat: "Demographic fairness testing
uses CLIP-proxy grouping, a coarse computational approximation pending human annotation."
The test result (PASS) at the time of submission should be stated.
