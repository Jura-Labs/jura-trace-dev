// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Pin how an action's softwareAgent is shown (BL-UX-001).
 *
 * C2PA 2.x writes softwareAgent as an object, and Jura Trace's own signer
 * emits that form, so every Trace-signed file used to read
 * "Tool: [object Object]" in the Content Credentials panel.
 */
import { describe, expect, it } from 'vitest';
import { formatSoftwareAgent } from './c2pa-labels';

describe('formatSoftwareAgent', () => {
  it('renders the object form Jura Trace itself signs with', () => {
    // Shape from src-tauri/src/c2pa.rs software_agent_map.
    const actions = JSON.parse(
      '{"actions":[{"action":"c2pa.created","softwareAgent":{"name":"Jura Trace","version":"1.1.0","operating_system":"macos"}}]}',
    ).actions;
    expect(formatSoftwareAgent(actions[0].softwareAgent)).toBe('Jura Trace 1.1.0');
  });

  it('renders the name alone when there is no version', () => {
    expect(formatSoftwareAgent({ name: 'Adobe Photoshop' })).toBe('Adobe Photoshop');
  });

  it('keeps the C2PA 1.x string form', () => {
    expect(formatSoftwareAgent('Adobe Lightroom 7.0')).toBe('Adobe Lightroom 7.0');
  });

  it('returns null rather than anything unreadable', () => {
    for (const agent of [undefined, null, '', '   ', {}, { version: '2.0' }, { name: 42 }, ['x'], 7]) {
      expect(formatSoftwareAgent(agent)).toBeNull();
    }
  });

  it('never prints [object Object] or JSON', () => {
    const shown = formatSoftwareAgent({ name: 'Tool', version: '1', extra: { a: 1 } });
    expect(shown).toBe('Tool 1');
    expect(shown).not.toMatch(/\[object|\{/);
  });
});
