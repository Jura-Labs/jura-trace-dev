// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * AiDetectorRow component unit tests.
 *
 * Sibling to DetectorRow.test.ts — covers the AI card row chrome
 * (the "Is this AI-generated?" panel).  Asserts the contract in
 * isolation; the integration is exercised by ai-card.spec.ts via
 * Playwright.
 */
import { describe, expect, it } from 'vitest';
import { render } from '@testing-library/svelte';
import AiDetectorRow from './AiDetectorRow.svelte';

describe('AiDetectorRow', () => {
	it('renders the detector name and score percentage', () => {
		const { getByText } = render(AiDetectorRow, {
			props: {
				name: 'GBM Deepfake',
				score: 0.42,
				suspicious: false,
			},
		});

		expect(getByText('GBM Deepfake')).toBeInTheDocument();
		expect(getByText('42%')).toBeInTheDocument();
	});

	it('rounds the score to the nearest whole percent', () => {
		const { getByText } = render(AiDetectorRow, {
			props: {
				name: 'Probe',
				score: 0.876,
				suspicious: true,
			},
		});

		// Math.round(0.876 * 100) === 88
		expect(getByText('88%')).toBeInTheDocument();
	});

	it('applies amber-light title class when suspicious', () => {
		const { container } = render(AiDetectorRow, {
			props: {
				name: 'GBMSuspicious',
				score: 0.78,
				suspicious: true,
			},
		});

		const title = container.querySelector('li span.text-sm');
		expect(title).toHaveTextContent('GBMSuspicious');
		expect(title).toHaveClass('text-amber-light');
	});

	it('applies neutral title class when not suspicious', () => {
		const { container } = render(AiDetectorRow, {
			props: {
				name: 'GBMClean',
				score: 0.05,
				suspicious: false,
			},
		});

		const title = container.querySelector('li span.text-sm');
		expect(title).toHaveTextContent('GBMClean');
		expect(title).not.toHaveClass('text-amber-light');
	});

	it('renders confidence chip when supplied', () => {
		const { getByText } = render(AiDetectorRow, {
			props: {
				name: 'Detector',
				score: 0.4,
				suspicious: false,
				confidence: 'high confidence',
			},
		});

		expect(getByText('high confidence')).toBeInTheDocument();
	});

	it('does not render confidence chip when omitted', () => {
		const { container } = render(AiDetectorRow, {
			props: {
				name: 'Detector',
				score: 0.4,
				suspicious: false,
			},
		});

		// The chip uses bg-gray-100 / border classes — confirm no element
		// matching that selector exists when confidence prop is absent.
		const chip = container.querySelector('span.bg-gray-100.dark\\:bg-graphite-light');
		expect(chip).toBeNull();
	});

	it('applies suspicious tint to the <li> wrapper', () => {
		const { container } = render(AiDetectorRow, {
			props: {
				name: 'D',
				score: 0.78,
				suspicious: true,
			},
		});

		const li = container.querySelector('li');
		expect(li).toHaveClass('bg-amber/[0.04]');
	});

	it('renders the warning icon when suspicious', () => {
		const { container } = render(AiDetectorRow, {
			props: {
				name: 'D',
				score: 0.78,
				suspicious: true,
			},
		});

		const warning = container.querySelector('svg path[d^="M10.29 3.86"]');
		expect(warning).not.toBeNull();
		const check = container.querySelector('svg polyline');
		expect(check).toBeNull();
	});

	it('renders the check icon when not suspicious', () => {
		const { container } = render(AiDetectorRow, {
			props: {
				name: 'D',
				score: 0.05,
				suspicious: false,
			},
		});

		const check = container.querySelector('svg polyline');
		expect(check).not.toBeNull();
		const warning = container.querySelector('svg path[d^="M10.29 3.86"]');
		expect(warning).toBeNull();
	});

	it('uses items-start layout (multi-line content column)', () => {
		const { container } = render(AiDetectorRow, {
			props: {
				name: 'D',
				score: 0.4,
				suspicious: false,
			},
		});

		// Distinguishing layout from DetectorRow's items-center: the
		// AI row uses items-start so its content column can wrap to
		// multi-line summary text + accordion below the title.
		const flexRow = container.querySelector('li > div.flex');
		expect(flexRow).toHaveClass('items-start');
	});

	describe('forensicScoreClass colour bands on score percentage', () => {
		it('uses malachite class for scores below 0.30', () => {
			const { getByText } = render(AiDetectorRow, {
				props: { name: 'X', score: 0.1, suspicious: false },
			});
			expect(getByText('10%')).toHaveClass('text-malachite-dark');
		});

		it('uses amber class for scores 0.30–0.59', () => {
			const { getByText } = render(AiDetectorRow, {
				props: { name: 'X', score: 0.45, suspicious: false },
			});
			expect(getByText('45%')).toHaveClass('text-amber-dark');
		});

		it('uses cinnabar class for scores 0.60 and above', () => {
			const { getByText } = render(AiDetectorRow, {
				props: { name: 'X', score: 0.78, suspicious: true },
			});
			expect(getByText('78%')).toHaveClass('text-cinnabar-dark');
		});
	});
});
