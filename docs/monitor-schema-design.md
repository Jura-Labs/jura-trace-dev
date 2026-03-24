# MONITOR Feature — SQLite Schema Design

**Migration file**: `src-tauri/migrations/003_monitor_tables.sql`
**Status**: Design spike — not yet wired into `db.rs`
**Related sprint**: Sprint 18 (S18-C2)

---

## Overview

The MONITOR feature lets users register URLs for periodic automated checking
and receive alerts when content integrity signals change (C2PA manifest
stripped, watermark missing, page content replaced, URL taken down).

Two tables are introduced:

| Table | Role |
|---|---|
| `monitor_urls` | One row per watched URL — configuration and latest-check snapshot |
| `monitor_events` | One row per check execution — full history and case management state |

---

## Table Purposes

### `monitor_urls`

The watchlist. Each row represents a user's intent to monitor a URL over
time. It holds:

- **Identity** — a UUID primary key, an optional link to the local asset
  catalogue (`asset_id`), the URL itself, and a human-readable label.
- **Scheduling configuration** — `check_frequency` controls how often the
  background worker should re-check the URL (`hourly`, `daily`, `weekly`).
- **Last-check snapshot** — denormalised columns (`last_checked_at`,
  `last_status`, `last_content_hash`, `last_c2pa_valid`,
  `last_watermark_match`) that let the MONITOR dashboard render the current
  state of every URL in a single table scan, without joining to
  `monitor_events`.
- **Enabled flag** — `enabled = 0` pauses checking without deleting history.

The `asset_id` foreign key is **nullable** because users will often want to
monitor URLs discovered in the wild — sites using their images without
permission — before (or instead of) having that asset locally protected.
`ON DELETE SET NULL` preserves the monitoring record if the catalogue asset
is later removed.

### `monitor_events`

The audit trail of every check. Each row is **immutable once written**:
case management fields (`case_status`, `case_notes`, `case_updated_at`)
are the only columns that should be updated after insert. All other
columns are written once at check time and never changed.

This table accumulates indefinitely. A typical deployment monitoring 50 URLs
daily will generate ~18 000 rows per year — well within SQLite's comfort zone.

---

## Event Types

| event_type | Trigger condition |
|---|---|
| `check_ok` | Content hash unchanged; C2PA and watermark signals nominal |
| `content_changed` | SHA-256 of fetched content differs from `monitor_urls.last_content_hash` |
| `url_missing` | HTTP 404, DNS resolution failure, or connection refused |
| `c2pa_stripped` | C2PA manifest was present at a previous check, now absent |
| `c2pa_invalid` | C2PA manifest present but signature fails verification |
| `watermark_found` | Watermark UUID successfully extracted with confidence above threshold |
| `watermark_missing` | URL is associated with a watermarked asset but watermark is no longer extractable |
| `error` | Unexpected exception during check (timeout, TLS error, etc.); see `detail_json` |

**Alerting logic**: Only `content_changed`, `url_missing`, `c2pa_stripped`,
`c2pa_invalid`, and `watermark_missing` should surface in the alert inbox.
`check_ok` and `watermark_found` are informational records that confirm
positive compliance.

**Transition detection**: The Rust check worker compares the current result
against `monitor_urls.last_status` and `last_c2pa_valid` to determine which
event type to record. For example, a C2PA manifest that was valid last week
and is absent today produces a `c2pa_stripped` event; the same absence on
the very first check produces only a `check_ok` or `error` depending on
whether the URL was expected to carry a manifest.

---

## Case Management Workflow

Each `monitor_events` row has a `case_status` field. The state machine is:

```
         ┌──────────────┐
  insert →│     new      │
         └──────┬───────┘
                │ user opens event
                ▼
         ┌──────────────┐
         │ investigating │
         └──────┬───────┘
                │              │
          resolved         escalated
                │              │
         ┌──────▼───────┐  ┌───▼──────────┐
         │   resolved   │  │  escalated   │
         └──────────────┘  └──────────────┘

  dismissed ← can be reached from any state
```

| Status | Meaning |
|---|---|
| `new` | Event recorded; no human has reviewed it yet |
| `investigating` | User has opened the event and is actively looking into it |
| `resolved` | User concluded the situation; no further action required |
| `escalated` | User flagged for legal review, DMCA notice, or external action |
| `dismissed` | User determined this was a false alarm or not actionable |

**Only alert events progress through this workflow.** `check_ok` events are
written with `case_status = 'check_ok'` (a terminal pseudo-state) and are
excluded from case indexes and inbox queries.

When `case_status` changes, the application must also set `case_updated_at`
to the current timestamp. This is enforced at the application layer (not via
a trigger) for simplicity, consistent with the existing codebase pattern.

---

## Index Rationale

### `idx_monitor_urls_asset`

```sql
CREATE INDEX IF NOT EXISTS idx_monitor_urls_asset
    ON monitor_urls(asset_id);
```

Supports the "show all monitored URLs for this asset" lookup on the PROTECT
detail page. Low write frequency; worth having for UX responsiveness.

### `idx_monitor_urls_enabled_checked`

```sql
CREATE INDEX IF NOT EXISTS idx_monitor_urls_enabled_checked
    ON monitor_urls(enabled, last_checked_at);
```

The background scheduler query — "give me the enabled URL that was checked
least recently" — is served entirely from this index:

```sql
SELECT url_id, url, check_frequency
FROM monitor_urls
WHERE enabled = 1
ORDER BY last_checked_at ASC NULLS FIRST
LIMIT 1;
```

`NULLS FIRST` ensures never-checked URLs are prioritised over recently-
checked ones. The compound index `(enabled, last_checked_at)` means SQLite
can seek to the `enabled = 1` partition and then walk it in
`last_checked_at` order without a sort.

### `idx_monitor_events_url_checked`

```sql
CREATE INDEX IF NOT EXISTS idx_monitor_events_url_checked
    ON monitor_events(url_id, checked_at);
```

The primary timeline query for the MONITOR detail view ("all events for URL
X, newest first") is the most frequent UI read. This covering index
eliminates the table lookup for the event list.

### `idx_monitor_events_type`

```sql
CREATE INDEX IF NOT EXISTS idx_monitor_events_type
    ON monitor_events(event_type);
```

Supports summary queries across all URLs: "how many `c2pa_stripped` events
in the last 30 days?" These are used for dashboard badge counts.

### `idx_monitor_events_case_active` (partial index)

```sql
CREATE INDEX IF NOT EXISTS idx_monitor_events_case_active
    ON monitor_events(case_status, checked_at)
    WHERE case_status NOT IN ('dismissed', 'check_ok');
```

The alert inbox query — "show me all open cases, newest first" — only ever
cares about active states. A partial index excluding `dismissed` and
`check_ok` rows keeps the index compact even after years of check history
accumulate. As `check_ok` events are expected to be the vast majority of
rows (most checks will find nothing wrong), this exclusion is especially
significant for index size.

---

## Query Patterns

### 1. Dashboard: list all active URLs with latest status

```sql
SELECT
    url_id,
    label,
    url,
    last_checked_at,
    last_status,
    last_c2pa_valid,
    last_watermark_match,
    enabled
FROM monitor_urls
WHERE enabled = 1
ORDER BY last_checked_at DESC;
```

This query is served from `monitor_urls` alone — no join required — because
the last-check state is denormalised into the parent row.

### 2. Detail view: event timeline for a URL

```sql
SELECT
    event_id,
    event_type,
    checked_at,
    content_hash,
    c2pa_valid,
    watermark_uuid,
    watermark_confidence,
    http_status,
    response_time_ms,
    case_status,
    case_notes,
    case_updated_at
FROM monitor_events
WHERE url_id = ?1
ORDER BY checked_at DESC
LIMIT ?2 OFFSET ?3;
```

Uses `idx_monitor_events_url_checked`. The `LIMIT`/`OFFSET` supports
pagination for URLs with long histories.

### 3. Alert inbox: all open cases

```sql
SELECT
    e.event_id,
    e.url_id,
    u.label,
    u.url,
    e.event_type,
    e.checked_at,
    e.case_status,
    e.case_notes
FROM monitor_events e
JOIN monitor_urls u ON u.url_id = e.url_id
WHERE e.case_status NOT IN ('dismissed', 'check_ok')
ORDER BY e.checked_at DESC;
```

The partial index `idx_monitor_events_case_active` covers the
`WHERE e.case_status NOT IN (...)` filter. The `JOIN` to `monitor_urls`
fetches the label and URL for display.

### 4. Badge counts: alerts by case status

```sql
SELECT case_status, COUNT(*) AS count
FROM monitor_events
WHERE case_status NOT IN ('dismissed', 'check_ok')
GROUP BY case_status;
```

Used to render the navigation badge (e.g. "3 new alerts") in the MONITOR
tab. The partial index makes this an index-only scan over the small set of
active cases.

### 5. Summary: alert events in the last 30 days by type

```sql
SELECT event_type, COUNT(*) AS count
FROM monitor_events
WHERE event_type != 'check_ok'
  AND checked_at >= datetime('now', '-30 days')
GROUP BY event_type
ORDER BY count DESC;
```

Uses `idx_monitor_events_type`. Suitable for a "last 30 days" breakdown card
on the MONITOR dashboard.

### 6. Scheduler: next URL due for a check

```sql
SELECT url_id, url, check_frequency, last_content_hash
FROM monitor_urls
WHERE enabled = 1
ORDER BY last_checked_at ASC NULLS FIRST
LIMIT 1;
```

The `(enabled, last_checked_at)` index serves this as a seek + first-row
read. The application compares `last_checked_at` against the current time
and `check_frequency` to decide whether to actually fire the check.

### 7. Transition detection: was C2PA valid on the previous check?

```sql
SELECT c2pa_valid
FROM monitor_events
WHERE url_id = ?1
  AND event_type != 'error'
ORDER BY checked_at DESC
LIMIT 1;
```

Used by the check worker to determine whether an absent C2PA manifest
represents a new `c2pa_stripped` event or was always absent. Uses
`idx_monitor_events_url_checked`.

---

## Schema Conventions Followed

This migration matches the patterns established in `db.rs`:

| Convention | Applied |
|---|---|
| UUID primary keys as `TEXT` | `url_id TEXT PRIMARY KEY`, `event_id TEXT PRIMARY KEY` |
| Timestamps as `TEXT NOT NULL DEFAULT (datetime('now'))` | `created_at`, `updated_at`, `checked_at` |
| Booleans as `INTEGER` (0/1/NULL) | `enabled`, `last_c2pa_valid`, `c2pa_valid`, etc. |
| `REFERENCES` declared inline | `url_id REFERENCES monitor_urls(url_id)` |
| Index naming: `idx_<table>_<suffix>` | All five indexes follow this pattern |
| No `STRICT` or `WITHOUT ROWID` | Consistent with existing tables |
| `CREATE TABLE IF NOT EXISTS` | All DDL is idempotent |

One divergence from the initial design spec: the `monitor_urls` table uses
`url_id` as the primary key name (not `id`) to follow the existing pattern
of `asset_id`, `log_id`, `fingerprint_id`. Similarly `monitor_events` uses
`event_id`. The `false_positive_reports` table uses plain `id`, which appears
to be an older inconsistency.

---

## Applying the Migration

Because the existing schema is managed inline in `db.rs` via `init_schema`,
this SQL file documents the intended DDL for the sprint implementation.

When wiring into `db.rs` the implementation should:

1. Add the `CREATE TABLE IF NOT EXISTS monitor_urls (...)` block to the
   `init_schema` `execute_batch` call, after the existing five tables.
2. Add the `CREATE TABLE IF NOT EXISTS monitor_events (...)` block immediately
   after.
3. Add the five new `CREATE INDEX IF NOT EXISTS` statements to the same
   `execute_batch` call.
4. No `ALTER TABLE` statements are needed — these are additive new tables.

For users who already have a database without these tables, SQLite's
`CREATE TABLE IF NOT EXISTS` is a safe no-op when the tables are absent,
and silently succeeds when they already exist.

---

## Future Considerations

### Layer 2 — Reverse image search results

When reverse image search is added (Sprint 19 or later), a third table can
record each discovered URL alongside similarity score and source engine:

```sql
CREATE TABLE IF NOT EXISTS monitor_reverse_image_hits (
    hit_id          TEXT PRIMARY KEY,
    asset_id        TEXT NOT NULL REFERENCES assets(asset_id) ON DELETE CASCADE,
    discovered_url  TEXT NOT NULL,
    source_engine   TEXT NOT NULL,   -- 'google', 'tineye', 'bing'
    similarity_score REAL,
    thumbnail_url   TEXT,
    page_title      TEXT,
    discovered_at   TEXT NOT NULL DEFAULT (datetime('now')),
    -- Link to a monitor_url if the user promotes this hit to the watchlist
    promoted_url_id TEXT REFERENCES monitor_urls(url_id) ON DELETE SET NULL
);
```

The `promoted_url_id` back-link allows the UI to show "this monitored URL
was first discovered via reverse image search" without a JOIN chain.

### Layer 3 — Federation matches

If Jura Trace later participates in a federated registry (see PROJECT_SPEC
Phase 4), incoming match notifications from other Jura instances would map
cleanly into `monitor_events` with:

- `event_type = 'federation_match'`
- `detail_json` carrying the remote instance URL, asset fingerprint, and
  confidence

No schema change would be required for basic federation — the `event_type`
column is unconstrained TEXT. A new `source_instance` column could be added
via `ALTER TABLE monitor_events ADD COLUMN source_instance TEXT` if a
dedicated federation index becomes necessary.

### Retention policy

For long-lived deployments (years of daily checks across many URLs), a
background retention job could prune old `check_ok` events:

```sql
DELETE FROM monitor_events
WHERE event_type = 'check_ok'
  AND checked_at < datetime('now', '-90 days');
```

Alert events (`case_status != 'check_ok'`) should never be auto-pruned —
they are part of the user's evidence trail and belong in the audit record.

### C2PA manifest content

The schema currently stores only a boolean (`c2pa_valid`). A future
enhancement could store the extracted manifest JSON in `detail_json` so the
MONITOR detail view can show exactly which C2PA claims changed between
checks, not just whether validity changed.
