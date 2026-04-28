.PHONY: dev dev-sidecar dev-tauri build check clean install test test-rust test-python test-playwright test-types test-ui-unit fmt lint release-check

# ── Development ────────────────────────────────────────────────────

# Start the Python ML sidecar (port 8200)
dev-sidecar:
	cd sidecar && JURA_SIDECAR_KEY="" uvicorn main:app --host 127.0.0.1 --port 8200 --reload

# Start the Tauri desktop app (includes SvelteKit dev server on port 1420)
dev-tauri:
	cargo tauri dev

# Quick start: prints the two commands to run in separate terminals
dev:
	@echo "═══════════════════════════════════════════════════════"
	@echo "  Jura Trace Development"
	@echo "═══════════════════════════════════════════════════════"
	@echo ""
	@echo "  Terminal 1:  make dev-sidecar"
	@echo "  Terminal 2:  make dev-tauri"
	@echo ""
	@echo "  Or use a process manager:"
	@echo "    overmind start -f Procfile.dev"
	@echo "═══════════════════════════════════════════════════════"

# ── Testing ────────────────────────────────────────────────────────

# Run all test suites
test: test-rust test-python test-types test-ui-unit
	@echo "All tests passed."

# Rust tests + clippy + fmt check
test-rust:
	cd src-tauri && cargo test
	cd src-tauri && cargo clippy -- -D warnings
	cd src-tauri && cargo fmt --check

# Python sidecar tests
test-python:
	cd sidecar && python -m pytest tests/ -v

# SvelteKit type check
test-types:
	cd ui && npx svelte-check

# Vitest component unit tests (Svelte 5 + @testing-library/svelte)
test-ui-unit:
	cd ui && npm test

# Playwright e2e tests (requires dev server running)
test-playwright:
	cd ui && npx playwright test

# Quick pre-commit check (< 15 seconds)
test-quick:
	cd src-tauri && cargo fmt --check
	cd src-tauri && cargo check
	cd ui && npx svelte-check

# ── Build ──────────────────────────────────────────────────────────

# Build production app (all platforms)
build:
	cd ui && npm run build
	cargo tauri build

# Pre-release sanity check
release-check:
	@echo "Checking release readiness..."
	cd src-tauri && cargo test
	cd src-tauri && cargo clippy -- -D warnings
	cd src-tauri && cargo fmt --check
	cd ui && npx svelte-check
	cd sidecar && python -m pytest tests/ -v
	@echo ""
	@echo "Version check:"
	@grep '"version"' src-tauri/tauri.conf.json | head -1
	@grep '^version' src-tauri/Cargo.toml | head -1
	@grep '"version"' ui/package.json | head -1
	@echo ""
	@echo "All checks passed. Ready to tag."

# ── Utilities ──────────────────────────────────────────────────────

# Install all dependencies
install:
	cd ui && npm install
	cd src-tauri && cargo fetch
	cd sidecar && pip install -r requirements.txt

# Format all code
fmt:
	cd src-tauri && cargo fmt
	cd ui && npx prettier --write "src/**/*.{svelte,ts,js,css}"

# Lint all code
lint:
	cd src-tauri && cargo clippy -- -D warnings
	cd ui && npx svelte-check

# Clean build artifacts
clean:
	cd ui && rm -rf node_modules .svelte-kit build
	cd src-tauri && cargo clean

# ── Docker (development stack) ─────────────────────────────────────

docker-up:
	docker compose up -d

docker-down:
	docker compose down

docker-build:
	docker compose build

docker-logs:
	docker compose logs -f
