// SPDX-License-Identifier: AGPL-3.0-or-later
// BL-CLAIM-005: the model card's UnivFD figures must be the shipped probe's.
// The card carried v9's numbers for five months after v10onnx shipped,
// because nothing tied the page to the model. This test does.
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { describe, expect, it } from 'vitest';

// The repository root, found from wherever vitest was started (ui/ locally
// and in CI). happy-dom leaves import.meta.url unusable for this.
let repo = process.cwd();
while (!existsSync(join(repo, 'models', 'univfd_probe_v10onnx_meta.json'))) {
	const up = dirname(repo);
	if (up === repo) throw new Error('repository root not found from ' + process.cwd());
	repo = up;
}
const read = (p: string) => readFileSync(join(repo, p), 'utf8');

const page = read('ui/src/routes/help/model-cards/+page.svelte');
const meta = JSON.parse(read('models/univfd_probe_v10onnx_meta.json'));
const detector = read('sidecar/app/services/clip_detector.py');

const pct = (x: number) => `${(x * 100).toFixed(2)}%`;
const cell = (v: string) => `font-mono">${v}</td>`;

describe('model card, UnivFD probe', () => {
	it('describes the probe the sidecar is pinned to', () => {
		expect(detector).toContain(meta.sha256);
	});

	it('shows that probe’s held-out figures', () => {
		const e = meta.evaluation;
		expect(page).toContain(cell(e.auc_roc.toFixed(4)));
		expect(page).toContain(cell(pct(e.fp_rate)));
		expect(page).toContain(cell(pct(e.ai_recall)));
		const n = e.tn + e.fp + e.fn + e.tp;
		expect(n).toBe(meta.training_corpus.n_test);
		expect(page).toContain(n.toLocaleString('en-GB'));
	});

	it('no longer shows v9’s', () => {
		expect(page).not.toContain(cell(pct(meta.v9_baseline.fp_rate)));
	});
});
