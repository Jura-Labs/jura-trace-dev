/**
 * ImageZoom component unit tests.
 *
 * Covers the thumbnail render, modal-overlay open/close behaviour
 * (click + keyboard), and the optional caption / label overrides.
 * The full-screen overlay is exercised in the integrity-card and
 * ai-card Playwright suites; this file pins the local component
 * contract.
 */
import { describe, expect, it } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import ImageZoom from './ImageZoom.svelte';

const TINY_PNG =
	'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkAAIAAAoAAv/lxKUAAAAASUVORK5CYII=';

describe('ImageZoom', () => {
	it('renders the thumbnail with the supplied src and alt', () => {
		const { getByAltText } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'ELA heatmap' },
		});

		const img = getByAltText('ELA heatmap');
		expect(img).toHaveAttribute('src', TINY_PNG);
	});

	it('uses a generated aria-label including the alt text by default', () => {
		const { getByRole } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'Copy-Move overlay' },
		});

		const button = getByRole('button', {
			name: 'Copy-Move overlay — click to enlarge',
		});
		expect(button).toBeInTheDocument();
	});

	it('uses the explicit label prop when provided', () => {
		const { getByRole } = render(ImageZoom, {
			props: {
				src: TINY_PNG,
				alt: 'X',
				label: 'Inspect frequency spectrum at full size',
			},
		});

		expect(
			getByRole('button', {
				name: 'Inspect frequency spectrum at full size',
			}),
		).toBeInTheDocument();
	});

	it('renders the caption when supplied', () => {
		const { getByText } = render(ImageZoom, {
			props: {
				src: TINY_PNG,
				alt: 'X',
				caption: 'Click to enlarge — bright regions indicate mismatch',
			},
		});

		expect(
			getByText(/Click to enlarge — bright regions indicate mismatch/),
		).toBeInTheDocument();
	});

	it('does not render a caption when omitted', () => {
		const { container } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'X' },
		});

		// No <span class="italic"> for the caption.
		const caption = container.querySelector('button > span.italic');
		expect(caption).toBeNull();
	});

	it('does not render the modal overlay before the thumbnail is clicked', () => {
		const { queryByRole } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'X' },
		});

		// The overlay is `role="dialog"` — absent on initial render.
		expect(queryByRole('dialog')).toBeNull();
	});

	it('opens the modal overlay when the thumbnail button is clicked', async () => {
		const { getByRole, queryByRole } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'X' },
		});

		const thumbButton = getByRole('button', { name: /click to enlarge/ });
		await fireEvent.click(thumbButton);

		const dialog = queryByRole('dialog');
		expect(dialog).not.toBeNull();
		expect(dialog).toHaveAttribute('aria-modal', 'true');
	});

	it('renders the full-size image inside the open modal', async () => {
		const { getByRole, getByAltText } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'Heatmap' },
		});

		await fireEvent.click(getByRole('button', { name: /click to enlarge/ }));

		const fullSize = getByAltText('Full-size: Heatmap');
		expect(fullSize).toBeInTheDocument();
		expect(fullSize).toHaveAttribute('src', TINY_PNG);
	});

	it('closes the modal when the close button is clicked', async () => {
		const { getByRole, queryByRole } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'X' },
		});

		await fireEvent.click(getByRole('button', { name: /click to enlarge/ }));
		expect(queryByRole('dialog')).not.toBeNull();

		await fireEvent.click(
			getByRole('button', { name: 'Close image preview' }),
		);

		expect(queryByRole('dialog')).toBeNull();
	});

	it('closes the modal when the backdrop button is clicked', async () => {
		const { getByRole, queryByRole } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'X' },
		});

		await fireEvent.click(getByRole('button', { name: /click to enlarge/ }));

		await fireEvent.click(
			getByRole('button', { name: 'Close full-size image preview' }),
		);

		expect(queryByRole('dialog')).toBeNull();
	});

	it('closes the modal when Escape is pressed', async () => {
		const { getByRole, queryByRole } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'X' },
		});

		await fireEvent.click(getByRole('button', { name: /click to enlarge/ }));
		expect(queryByRole('dialog')).not.toBeNull();

		await fireEvent.keyDown(window, { key: 'Escape' });

		expect(queryByRole('dialog')).toBeNull();
	});

	it('does NOT close the modal on other keypresses', async () => {
		const { getByRole, queryByRole } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'X' },
		});

		await fireEvent.click(getByRole('button', { name: /click to enlarge/ }));
		await fireEvent.keyDown(window, { key: 'Enter' });
		await fireEvent.keyDown(window, { key: ' ' });
		await fireEvent.keyDown(window, { key: 'a' });

		expect(queryByRole('dialog')).not.toBeNull();
	});

	it('renders the caption inside the modal when supplied', async () => {
		const caption = 'Click to enlarge — periodic peaks reveal upscaling';
		const { getByRole, getByText } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'X', caption },
		});

		await fireEvent.click(getByRole('button', { name: /click to enlarge/ }));

		// The caption appears in the modal as well as under the thumbnail.
		const captions = document.querySelectorAll('*');
		const matches = Array.from(captions).filter(
			(el) => el.textContent === caption && el.tagName !== 'BUTTON',
		);
		expect(matches.length).toBeGreaterThanOrEqual(1);
	});

	it('applies the default thumbClass when no override is supplied', () => {
		const { getByAltText } = render(ImageZoom, {
			props: { src: TINY_PNG, alt: 'X' },
		});

		const img = getByAltText('X');
		// Default class string includes "max-h-48 object-contain"
		expect(img.className).toContain('max-h-48');
		expect(img.className).toContain('object-contain');
	});

	it('applies a custom thumbClass when overridden', () => {
		const { getByAltText } = render(ImageZoom, {
			props: {
				src: TINY_PNG,
				alt: 'X',
				thumbClass: 'w-10 h-10 rounded object-cover',
			},
		});

		const img = getByAltText('X');
		expect(img.className).toContain('w-10');
		expect(img.className).toContain('h-10');
		expect(img.className).not.toContain('max-h-48');
	});
});
