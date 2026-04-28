import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],

	// Tauri expects a fixed port
	server: {
		port: 1420,
		strictPort: true
	},

	// Env prefix for Tauri
	envPrefix: ['VITE_', 'TAURI_'],

	// @ts-expect-error vitest extends vite config at runtime
	test: {
		// happy-dom is lighter and faster than jsdom for component
		// rendering; @testing-library/svelte requires a DOM-like
		// environment.
		environment: 'happy-dom',
		// Force the browser resolution conditions when running tests
		// so Svelte's `mount()` is available — the SvelteKit Vite
		// plugin defaults to the server resolution otherwise, which
		// throws `lifecycle_function_unavailable` for component
		// rendering.
		alias: {
			// Resolve Svelte to its browser entry for component tests.
		},
		server: {
			deps: {
				// inline the svelte runtime for the test environment
				inline: ['@testing-library/svelte'],
			},
		},
		// Load @testing-library/jest-dom matcher extensions before each
		// test file (toBeInTheDocument, toHaveTextContent, etc.).
		setupFiles: ['./src/test/setup.ts'],
		// Component unit tests live alongside source as `*.test.ts`.
		// Playwright e2e tests under tests/ are excluded explicitly so
		// `npm test` does not try to run them through vitest.
		include: ['src/**/*.{test,spec}.{ts,js}'],
		exclude: ['tests/**', 'node_modules/**', '.svelte-kit/**'],
		// Keep this so `npm test` does not fail in branches without
		// component tests yet.
		passWithNoTests: true
	},
	resolve: {
		// When running under vitest in component-test mode, prefer the
		// browser conditions for any package that ships dual-mode
		// entries.  Without this, Svelte 5 resolves to the server
		// entry where `mount()` is intentionally unavailable.
		conditions: process.env.VITEST ? ['browser'] : undefined,
	}
});
