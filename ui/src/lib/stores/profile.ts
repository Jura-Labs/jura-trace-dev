// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Deployment profile store for Jura Trace settings.
 *
 * A deployment profile captures a named snapshot of Ollama configuration
 * and verify mode so operators can switch between environments (e.g. local
 * development vs. institutional server) without manually re-entering values.
 *
 * Storage: localStorage under STORAGE_KEY as a JSON array.
 * Limit: MAX_PROFILES profiles per installation.
 */

export interface DeploymentProfile {
  /** Unique identifier (crypto.randomUUID or Date.now fallback) */
  id: string;
  /** Human-readable label, 1–50 characters, trimmed */
  name: string;
  /** Ollama base URL, e.g. http://localhost:11434 */
  ollamaUrl: string;
  /** Ollama vision model identifier, e.g. llava:7b */
  visionModel: string;
  /** Ollama text model identifier, e.g. qwen2.5:7b-instruct */
  textModel: string;
  /** Default pipeline depth when verifying assets */
  defaultVerifyMode: 'standard' | 'deep' | 'archival';
  /** ISO 8601 creation timestamp */
  createdAt: string;
}

const STORAGE_KEY = 'jura-profiles';

/** Maximum number of saved profiles permitted per installation */
export const MAX_PROFILES = 10;

// ── Individual setting localStorage keys (mirrors settings/+page.svelte) ──

const KEY_OLLAMA_URL   = 'jura-ollama-url';
const KEY_VISION_MODEL = 'jura-vision-model';
const KEY_TEXT_MODEL   = 'jura-text-model';

// ── CRUD helpers ───────────────────────────────────────────────────────────

/**
 * Load all saved profiles from localStorage.
 * Returns an empty array when none exist or on parse failure.
 */
export function loadProfiles(): DeploymentProfile[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed as DeploymentProfile[];
  } catch {
    return [];
  }
}

/**
 * Persist a single profile, inserting or replacing by id.
 * Silently ignores writes when the maximum has been reached and the
 * profile is new (id not already present).
 */
export function saveProfile(p: DeploymentProfile): void {
  if (typeof localStorage === 'undefined') return;
  const profiles = loadProfiles();
  const existingIndex = profiles.findIndex((x) => x.id === p.id);
  if (existingIndex !== -1) {
    // Replace existing
    profiles[existingIndex] = p;
  } else {
    if (profiles.length >= MAX_PROFILES) return;
    profiles.push(p);
  }
  localStorage.setItem(STORAGE_KEY, JSON.stringify(profiles));
}

/**
 * Remove a profile by id. No-op if the id is not found.
 */
export function deleteProfile(id: string): void {
  if (typeof localStorage === 'undefined') return;
  const profiles = loadProfiles().filter((p) => p.id !== id);
  localStorage.setItem(STORAGE_KEY, JSON.stringify(profiles));
}

/**
 * Apply a profile by writing each field to its individual localStorage key.
 * The settings page reads these keys on mount, so the caller is responsible
 * for updating its own reactive state after calling this function.
 */
export function applyProfile(p: DeploymentProfile): void {
  if (typeof localStorage === 'undefined') return;
  localStorage.setItem(KEY_OLLAMA_URL,   p.ollamaUrl);
  localStorage.setItem(KEY_VISION_MODEL, p.visionModel);
  localStorage.setItem(KEY_TEXT_MODEL,   p.textModel);
}

/**
 * Generate a unique profile id.
 * Prefers crypto.randomUUID when available; falls back to Date.now.
 */
export function generateProfileId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return String(Date.now());
}

/**
 * Validate a profile name.
 * Returns an error message string, or null when valid.
 */
export function validateProfileName(name: string): string | null {
  const trimmed = name.trim();
  if (trimmed.length === 0) return 'Profile name must not be empty.';
  if (trimmed.length > 50) return 'Profile name must be 50 characters or fewer.';
  return null;
}

/**
 * Format an ISO 8601 timestamp for display.
 * Returns a short locale date string, e.g. "18 Mar 2026".
 */
export function formatProfileDate(isoString: string): string {
  try {
    return new Date(isoString).toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
    });
  } catch {
    return isoString;
  }
}
