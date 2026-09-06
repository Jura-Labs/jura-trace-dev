# Jura Trace backlog

This directory is the system of record for Jura Trace backlog items, in the
same shape ROOTED uses. One file per item, committed to git, written for
somebody who has not seen the code.

The convention is in `AGENTS.md` under "Backlog convention". The short
version:

- A BL file states what is wrong, how it showed up, why it matters, what
  would fix it and what not to do, with file paths and line numbers.
- Implement on a branch, never on `main`.
- When it merges, append `Resolved: YYYY-MM-DD <commit> — what shipped` to
  the file. Do not delete it. The history is the point.
- Every merge gets a line in `~/jura-brain/log.md` with a deliverable ID.

## Open items

| Item | What it is | Severity |
|---|---|---|
| [BL-REL-002](BL-REL-002-updater-manifest-carries-urls-not-signatures.md) | The updater manifest holds URLs where signatures belong, so no v1.0.0 install can update | Highest |
| [BL-CI-001](BL-CI-001-test-gate-has-not-run-since-april.md) | The test gate has not run since 30 April and only fires on pull requests | High |
| [BL-CI-002](BL-CI-002-osv-scanner-has-scanned-nothing-since-july.md) | The weekly supply-chain scan has scanned nothing since July | High |
| [BL-DEPS-001](BL-DEPS-001-dependabot-queue-is-saturated.md) | All four dependency ecosystems are at the five-PR cap, so nothing new can be reported | High |
| [BL-CLAIM-001](BL-CLAIM-001-nothing-leaves-the-device-is-nearly-true.md) | "No data is uploaded to external servers" is nearly true, and nearly is not good enough | High |
| [BL-DEPS-002](BL-DEPS-002-frontend-advisories-one-ships.md) | Twelve frontend advisories, of which one reaches a user | Medium |
| [BL-REL-001](BL-REL-001-stray-model-backup-in-the-bundle.md) | A stray classifier backup sits inside the bundled resources glob | Low |
| [BL-UX-001](BL-UX-001-tool-object-object-in-the-c2pa-panel.md) | The C2PA panel shows "Tool: [object Object]" | Low to fix |
| [BL-CLAIM-002](BL-CLAIM-002-the-app-makes-two-promises-the-release-did-not-keep.md) | Two promises shipped inside the app that the release did not keep | Medium |
| [BL-DOC-001](BL-DOC-001-pdfs-already-half-work-and-the-help-page-denies-it.md) | PDFs already half-work by drag and drop, and the help page denies it | Medium |
| [BL-WM-001](BL-WM-001-watermarking-is-two-bugs-and-a-bundle-problem.md) | What enabling watermarking actually requires, and the robustness cliff | Scope risk |
| [BL-DEPS-003](BL-DEPS-003-four-reachable-advisories-in-the-shipped-binary.md) | Four reachable advisories in the shipped product, found the first time anyone looked | High |
| [BL-TEST-001](BL-TEST-001-nothing-tests-the-installed-product.md) | Nothing tests the installed product; this workflow IS the BL-REL-002 release gate | High, v1.1.0 floor |
| [BL-SILENT-001](BL-SILENT-001-mechanisms-that-report-success-while-doing-nothing.md) | The class behind five defects: mechanisms reporting success while doing nothing | High as a class |
| [BL-CLAIM-003](BL-CLAIM-003-the-pdf-report-asserts-checks-that-never-ran.md) | The exported forensic report asserts OCSP/CRL checks that never run | High, legal |
| [BL-API-001](BL-API-001-the-headless-api-is-documented-and-does-not-exist.md) | The headless API is documented and does not exist; it gates the CLI | Medium |
| [BL-SIZE-001](BL-SIZE-001-242mb-of-download-computes-10kb-of-constants.md) | 242 MB of every download computes 10 KB of constants | High for adoption |

The first four are one story. There is no working channel through which a
newly disclosed vulnerability reaches anybody, and no working channel
through which a fix reaches a user. Reading them in the order above is the
order they should be repaired in.

## Two standing instructions from Paul, 3 September 2026

- **This branch stays unmerged** until D1 to D3 in
  `~/jura-brain/inbox/2026-09-03-trace-plane-triage.md` are answered.
- **v1.0.1 is not cut** before BL-REL-002 has a scheduled fix. That item
  also does not close on the code fix alone: it is paired with a
  re-download notice, because at least 21 installs cannot be reached by any
  updater repair.

## Where the rest of the backlog is, for now

Plane's `JTV` project holds 248 work items, 156 of them open, none touched
since 22 June 2026. It is the historical record of everything planned
between March and June and is not being used day to day.

A proposal for what carries over into this directory, and whether Plane
retires for Trace as it has for ROOTED, is with Paul at
`~/jura-brain/inbox/2026-09-03-trace-plane-triage.md`. Nothing in Plane has
been or will be deleted.

Until that is decided, the eight items above are the ones found by reading
the code and the live systems on 3 September 2026, not by mining Plane.
