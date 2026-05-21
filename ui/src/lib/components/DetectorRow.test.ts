// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * DetectorRow component unit tests.
 *
 * Vitest + @testing-library/svelte (happy-dom).  Run with `npm test`.
 *
 * Sister coverage to the integrity-card.spec.ts Playwright suite —
 * unit tests assert the component contract in isolation (faster
 * feedback, no Tauri dev server needed) while the e2e suite asserts
 * the integration through the verify page.  Both are valuable: keep
 * unit tests for prop-matrix coverage of new variants, e2e tests for
 * regressions visible to a user.
 */
import { describe, expect, it } from 'vitest';
import { render } from '@testing-library/svelte';
import DetectorRow from './DetectorRow.svelte';

describe('DetectorRow', () => {
	it('renders the detector name and score percentage', () => {
		const { getByText } = render(DetectorRow, {
			props: {
				name: 'ELA Detector',
				score: 0.42,
				suspicious: false,
				helpAnchor: 'ela',
			},
		});

		expect(getByText('ELA Detector')).toBeInTheDocument();
		expect(getByText('42%')).toBeInTheDocument();
	});

	it('rounds the score to the nearest whole percent', () => {
		const { getByText } = render(DetectorRow, {
			props: {
				name: 'Noise',
				score: 0.314159,
				suspicious: false,
				helpAnchor: 'noise-pattern',
			},
		});

		// Math.round(0.314159 * 100) === 31
		expect(getByText('31%')).toBeInTheDocument();
	});

	it('applies the amber-dark title class when suspicious (light-mode WCAG fix)', () => {
		const { getByText } = render(DetectorRow, {
			props: {
				name: 'Copy-Move',
				score: 0.62,
				suspicious: true,
				helpAnchor: 'copy-move',
			},
		});

		const title = getByText('Copy-Move');
		expect(title).toHaveClass('text-amber-dark');
	});

	it('applies neutral title class when not suspicious', () => {
		// Use a unique name to avoid getByText collisions with the
		// generated help-link aria-label / title attribute that
		// happen to contain the detector name.
		const { container } = render(DetectorRow, {
			props: {
				name: 'CleanCopyMove',
				score: 0.05,
				suspicious: false,
				helpAnchor: 'copy-move',
			},
		});

		// The title span carries the `text-sm` class — query by it
		// directly to disambiguate from any aria-label text.
		const title = container.querySelector('li span.text-sm');
		expect(title).toHaveTextContent('CleanCopyMove');
		expect(title).not.toHaveClass('text-amber-dark');
	});

	it('renders a help link to /help/forensic-detectors with the anchor', () => {
		const { getByRole } = render(DetectorRow, {
			props: {
				name: 'JPEG Ghost',
				score: 0.4,
				suspicious: false,
				helpAnchor: 'jpeg-ghost',
				helpLabel: 'Learn about JPEG Ghost',
			},
		});

		const link = getByRole('link', { name: 'Learn about JPEG Ghost' });
		expect(link).toBeInTheDocument();
		expect(link).toHaveAttribute('href', '/help/forensic-detectors#jpeg-ghost');
	});

	it('falls back to a generated help label when none supplied', () => {
		const { getByRole } = render(DetectorRow, {
			props: {
				name: 'Splice Boundary',
				score: 0.3,
				suspicious: false,
				helpAnchor: 'splice-boundary',
			},
		});

		// Default fallback is "What does {name} check?"
		expect(
			getByRole('link', { name: 'What does Splice Boundary check?' }),
		).toBeInTheDocument();
	});

	it('renders alwaysVisibleHint when supplied', () => {
		const { getByText } = render(DetectorRow, {
			props: {
				name: 'Noise Pattern Analysis',
				score: 0.31,
				suspicious: false,
				helpAnchor: 'noise-pattern',
				alwaysVisibleHint:
					'Measures whether noise distribution is uniform across the photo.',
			},
		});

		expect(
			getByText(
				/Measures whether noise distribution is uniform across the photo/,
			),
		).toBeInTheDocument();
	});

	it('does not render alwaysVisibleHint when omitted', () => {
		const { container } = render(DetectorRow, {
			props: {
				name: 'ELA',
				score: 0.42,
				suspicious: false,
				helpAnchor: 'ela',
			},
		});

		// No <p> element should be present in the row body when no
		// hint is supplied (the children slot is empty too).
		expect(container.querySelector('li > p')).toBeNull();
	});

	it('applies the suspicious tint to the <li> wrapper when suspicious', () => {
		const { container } = render(DetectorRow, {
			props: {
				name: 'ELA',
				score: 0.78,
				suspicious: true,
				helpAnchor: 'ela',
			},
		});

		const li = container.querySelector('li');
		expect(li).toHaveClass('bg-amber/[0.04]');
	});

	it('does not apply the suspicious tint when not suspicious', () => {
		const { container } = render(DetectorRow, {
			props: {
				name: 'ELA',
				score: 0.05,
				suspicious: false,
				helpAnchor: 'ela',
			},
		});

		const li = container.querySelector('li');
		expect(li).not.toHaveClass('bg-amber/[0.04]');
	});

	it('renders the warning icon path when suspicious', () => {
		const { container } = render(DetectorRow, {
			props: {
				name: 'ELA',
				score: 0.78,
				suspicious: true,
				helpAnchor: 'ela',
			},
		});

		// The warning icon is a path that starts with M10.29 3.86 (the
		// triangle).  The check icon is a polyline.
		const warning = container.querySelector('svg path[d^="M10.29 3.86"]');
		expect(warning).not.toBeNull();
		const check = container.querySelector('svg polyline');
		expect(check).toBeNull();
	});

	it('renders the check icon when not suspicious', () => {
		const { container } = render(DetectorRow, {
			props: {
				name: 'ELA',
				score: 0.05,
				suspicious: false,
				helpAnchor: 'ela',
			},
		});

		const check = container.querySelector('svg polyline');
		expect(check).not.toBeNull();
		const warning = container.querySelector('svg path[d^="M10.29 3.86"]');
		expect(warning).toBeNull();
	});

	describe('forensicScoreClass colour bands on score percentage', () => {
		it('uses malachite class for scores below 0.30', () => {
			const { getByText } = render(DetectorRow, {
				props: { name: 'X', score: 0.1, suspicious: false, helpAnchor: 'ela' },
			});
			const score = getByText('10%');
			expect(score).toHaveClass('text-malachite-dark');
		});

		it('uses amber class for scores 0.30–0.59', () => {
			const { getByText } = render(DetectorRow, {
				props: { name: 'X', score: 0.45, suspicious: false, helpAnchor: 'ela' },
			});
			const score = getByText('45%');
			expect(score).toHaveClass('text-amber-dark');
		});

		it('uses cinnabar class for scores 0.60 and above', () => {
			const { getByText } = render(DetectorRow, {
				props: { name: 'X', score: 0.75, suspicious: true, helpAnchor: 'ela' },
			});
			const score = getByText('75%');
			expect(score).toHaveClass('text-cinnabar-dark');
		});
	});
});
