/**
 * Vitest setup — runs once before each test file.
 *
 * Extends `expect` with the @testing-library/jest-dom matchers so
 * tests can write `expect(node).toBeInTheDocument()` and friends,
 * and registers automatic component cleanup between tests so
 * `render(...)` calls do not pollute `document.body` across tests
 * (which manifests as "multiple elements found" assertion errors).
 */
import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/svelte';
import { afterEach } from 'vitest';

afterEach(() => {
	cleanup();
});
