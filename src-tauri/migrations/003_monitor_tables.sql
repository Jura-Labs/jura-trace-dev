-- Migration 003: MONITOR feature — URL watchlist and alert management
--
-- Adds two tables to support the MONITOR feature:
--   monitor_urls    — the set of URLs a user has registered for periodic checking
--   monitor_events  — individual check results, one row per check per URL
--
-- Design notes:
--   * Follows existing db.rs conventions: TEXT primary keys (UUIDs), datetime('now')
--     for default timestamps, INTEGER 0/1 for booleans, snake_case column names.
--   * No STRICT mode or WITHOUT ROWID — consistent with existing tables.
--   * Foreign key to assets(asset_id) is nullable: users may monitor URLs that are
--     not yet in their local catalogue (e.g. they spotted their image on a third-
--     party site before protecting it).
--   * monitor_events is append-only. Case management state (case_status,
--     case_notes) lives on the event row so every status transition is
--     auditable without a separate case table. The case_updated_at column
--     records the most recent status change timestamp.
--   * ON DELETE CASCADE on monitor_events.url_id ensures that removing a
--     monitored URL also removes its event history — appropriate for a local-
--     first tool where the user controls retention.
--
-- Apply inside a transaction so the migration is atomic:
--   BEGIN;
--   <contents of this file>
--   COMMIT;

-- ── 1. monitor_urls ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS monitor_urls (
    -- Identity
    url_id          TEXT PRIMARY KEY,           -- UUID (v4, generated in Rust)
    asset_id        TEXT                        -- FK to assets(asset_id); nullable
                        REFERENCES assets(asset_id)
                        ON DELETE SET NULL,
    url             TEXT NOT NULL,              -- Absolute URL being monitored
    label           TEXT,                       -- User-supplied human label

    -- Scheduling
    check_frequency TEXT NOT NULL DEFAULT 'daily',
        -- Allowed values: 'hourly' | 'daily' | 'weekly'
        -- Enforced at the application layer; a CHECK constraint would prevent
        -- adding new frequencies without a schema migration.

    -- Last-check snapshot (denormalised for quick dashboard rendering)
    last_checked_at     TEXT,                   -- ISO 8601 timestamp of most recent check
    last_status         TEXT,                   -- 'ok' | 'changed' | 'missing' | 'error'
    last_content_hash   TEXT,                   -- SHA-256 hex of content at last check
    last_c2pa_valid     INTEGER,                -- 0 = invalid, 1 = valid, NULL = not checked
    last_watermark_match INTEGER,               -- 0 = not found/mismatch, 1 = matched, NULL = not checked

    -- State
    enabled         INTEGER NOT NULL DEFAULT 1, -- 1 = active, 0 = paused

    -- Timestamps
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ── 2. monitor_events ────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS monitor_events (
    -- Identity
    event_id        TEXT PRIMARY KEY,           -- UUID (v4, generated in Rust)
    url_id          TEXT NOT NULL
                        REFERENCES monitor_urls(url_id)
                        ON DELETE CASCADE,

    -- What happened
    event_type      TEXT NOT NULL,
        -- 'check_ok'          — content unchanged, all signals nominal
        -- 'content_changed'   — SHA-256 differs from previous check
        -- 'url_missing'       — HTTP 404 / DNS failure / connection refused
        -- 'c2pa_stripped'     — C2PA manifest present before, absent now
        -- 'c2pa_invalid'      — C2PA manifest present but signature invalid
        -- 'watermark_found'   — watermark UUID successfully extracted
        -- 'watermark_missing' — watermark expected but not found
        -- 'error'             — unexpected error during check (see detail_json)

    -- Check snapshot
    checked_at          TEXT NOT NULL DEFAULT (datetime('now')),
    content_hash        TEXT,                   -- SHA-256 hex of content at this check
    c2pa_valid          INTEGER,                -- 0 / 1 / NULL
    watermark_uuid      TEXT,                   -- Extracted watermark UUID, if any
    watermark_confidence REAL,                  -- Extraction confidence 0.0–1.0

    -- HTTP diagnostics
    http_status         INTEGER,                -- HTTP status code (200, 404, etc.)
    response_time_ms    INTEGER,                -- Round-trip time in milliseconds

    -- Case management
    --
    -- State machine:
    --   new → investigating → resolved
    --                      ↘ escalated
    --       → dismissed    (can be set from any state)
    --
    -- 'new'          — event has just been recorded, no human review yet
    -- 'investigating'— user has opened the event and is looking into it
    -- 'resolved'     — user has concluded action; no further follow-up needed
    -- 'escalated'    — user has flagged for legal/external follow-up
    -- 'dismissed'    — user determined this was a false alarm or not actionable
    --
    case_status     TEXT NOT NULL DEFAULT 'new',
    case_notes      TEXT,                       -- Free-text notes from the user
    case_updated_at TEXT,                       -- Timestamp of last case_status change

    -- Extensibility
    detail_json     TEXT                        -- JSON: error messages, response headers, etc.
);

-- ── 3. Indexes ───────────────────────────────────────────────────────────────

-- Look up all monitored URLs for a given asset (catalogue → monitor link).
CREATE INDEX IF NOT EXISTS idx_monitor_urls_asset
    ON monitor_urls(asset_id);

-- Scheduler query: find enabled URLs ordered by how long ago they were checked,
-- so the worker can pick the most overdue URL first.
CREATE INDEX IF NOT EXISTS idx_monitor_urls_enabled_checked
    ON monitor_urls(enabled, last_checked_at);

-- Primary event timeline for a URL (the most common UI query).
CREATE INDEX IF NOT EXISTS idx_monitor_events_url_checked
    ON monitor_events(url_id, checked_at);

-- Alert inbox: all events of a given type across all URLs (e.g. all
-- 'c2pa_stripped' events to build a "stripped this week" count).
CREATE INDEX IF NOT EXISTS idx_monitor_events_type
    ON monitor_events(event_type);

-- Case management inbox: active cases only. The partial index excludes
-- 'dismissed' rows so the index stays small as historical events accumulate.
-- 'check_ok' events never enter case workflows so they are also excluded.
CREATE INDEX IF NOT EXISTS idx_monitor_events_case_active
    ON monitor_events(case_status, checked_at)
    WHERE case_status NOT IN ('dismissed', 'check_ok');
