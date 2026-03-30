---
title: "Jura Trace v0.9.0-rc.3 — Pilot Tester Welcome Email Template"
description: "Ready-to-send email template for inviting pilot testers to evaluate Jura Trace v0.9.0-rc.3. Warm and non-technical. Customise the highlighted fields before sending."
last-updated: 29 March 2026
status: internal
audience: Pilot coordinator (Juralabs) — customise before sending
---

# Pilot Tester Welcome Email — Template

**Instructions for the coordinator.** Before sending, replace all fields marked with `[SQUARE BRACKETS]` with the correct values for each tester. Remove this instruction block. Send from paul@juralabs.org.

---

## Subject line

```
You're in — Jura Trace pilot testing starts now
```

---

## Email body

---

Dear [TESTER NAME],

Thank you for agreeing to help us test Jura Trace. We are a small team, and your time and observations matter more than you might think. What you find over the next two weeks will directly influence what we fix, what we clarify, and what we prioritise before the public release.

**What is Jura Trace?**

Jura Trace is a desktop application that helps you verify whether images, videos, and documents are authentic or AI-generated. It also lets you protect your own content by embedding cryptographic provenance records and invisible watermarks. Everything runs on your computer — no images are sent anywhere, no account is required, and nothing leaves your machine.

---

**Downloading the application**

You are on [PLATFORM: macOS / Windows], so download this file:

[CHOOSE ONE AND DELETE THE OTHER:]

> **macOS:**
> `Jura.Trace_0.9.0_aarch64.dmg` (258 MB)
> Download: https://github.com/juralabs/jura-archive/releases/tag/v0.9.0-rc.3

> **Windows:**
> `Jura.Trace_0.9.0_x64-setup.exe` (297 MB)
> Download: https://github.com/juralabs/jura-archive/releases/tag/v0.9.0-rc.3

---

**Your complete guide**

I have attached the Pilot Tester Guide to this email [OR: linked it here: [LINK]]. It walks you through installation, your first verification, and the Protect workflow, step by step — no technical background needed. It also explains what to do if you see a security warning during installation (this is expected for a pre-release build, and the guide explains exactly how to proceed).

Please read at least the first few sections before you start.

---

**What we are asking you to do**

Work through the structured test session described in the guide. It takes approximately 30 minutes and covers:

- Installing the application and getting it running
- Verifying an image (the core feature — this is the most important thing to try)
- Protecting a piece of content with a provenance record and watermark
- Exploring the help documentation

You do not need to complete everything in one sitting. The test session can be spread across the two-week period if that suits you better.

**When you have finished**, send your observations — however rough or informal — to paul@juralabs.org. The guide includes a five-question feedback form if you would like a structure to follow, but a few sentences in plain email is equally useful.

---

**Known limitations for this build**

This is a release candidate, not the finished product. A few things to be aware of:

- **Security warnings on installation are expected.** Because this build has not yet been submitted for code-signing, macOS and Windows will show warnings when you install it. These are standard caution messages for unsigned software — not a sign of anything wrong. The pilot guide explains how to proceed past them safely.
- **The forensic analysis engine starts automatically.** In previous builds, it had to be started separately. In this RC it should launch on its own. If the analysis engine shows as Offline in Settings, quit the application and relaunch it.
- **Ollama and AI features are optional.** The AI-assisted features (image descriptions, claim verification) require a separate free tool called Ollama. You do not need it for any of the core testing tasks. The guide covers it in an optional section if you want to explore it.
- **There is no Linux build in this release candidate.** Linux support is planned for v1.0.
- **This is v0.9.0 — some rough edges are expected.** If you find something that seems wrong or broken, please tell us. That is exactly why you are here.

---

**Two weeks**

The pilot runs until **[DATE: ADD DATE 14 DAYS FROM SEND DATE]**. I will check in with you on Day 1 to confirm the installation went smoothly, and again on Day 7 to see how you are getting on. There is no pressure to complete everything before then — but if you run into trouble at any point, please email me straight away.

---

**Contact**

All feedback, questions, and bug reports:

> Paul Griffiths
> Juralabs Community Interest Company (UK)
> paul@juralabs.org
> Response within 24 hours

---

Thank you again for your time. The work you put in over these two weeks will leave a mark on something we hope will be genuinely useful to journalists, archivists, researchers, and anyone who needs to know what is real.

Warm regards,

Paul Griffiths
Juralabs Community Interest Company (UK)
juralabs.org

---

*Jura Trace v0.9.0 is a pre-release evaluation build. It is licensed under PolyForm Noncommercial 1.0.0 and must not be used for commercial work. All processing occurs locally on your device — no data is transmitted to Juralabs or any external service.*

---

## Coordinator Notes

**Customisation checklist before sending:**

- [ ] Replace `[TESTER NAME]` with the tester's first name
- [ ] Replace `[PLATFORM: macOS / Windows]` with their platform
- [ ] Keep only the relevant download block (macOS or Windows) — delete the other
- [ ] Replace `[DATE: ADD DATE 14 DAYS FROM SEND DATE]` with the correct closing date
- [ ] Replace `[OR: linked it here: [LINK]]` with the correct attachment note or link
- [ ] Remove this Coordinator Notes section before sending
- [ ] Remove the front matter block before sending (the `---` YAML block at the top of this file)

**Subject line variants** (use the default above; these are alternatives if tone needs adjusting):

- `Jura Trace pilot — you are in, here is everything you need`
- `Your Jura Trace pilot build is ready to download`
- `Jura Trace v0.9.0 pilot — welcome and next steps`
