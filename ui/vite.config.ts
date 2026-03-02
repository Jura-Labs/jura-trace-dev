import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],

	// Tauri expects a fixed port
	server: {
		port: 5173,
		strictPort: true
	},

	// Env prefix for Tauri
	envPrefix: ['VITE_', 'TAURI_'],

	// @ts-expect-error vitest extends vite config at runtime
	test: {
		passWithNoTests: true
	}
});
