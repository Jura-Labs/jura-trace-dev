/**
 * Vitest setup — runs once before each test file.
 *
 * Extends `expect` with the @testing-library/jest-dom matchers so
 * tests can write `expect(node).toBeInTheDocument()` and friends.
 */
import '@testing-library/jest-dom/vitest';
