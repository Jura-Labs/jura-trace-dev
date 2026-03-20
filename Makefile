.PHONY: dev build check clean install docker-up docker-down docker-build docker-logs agents agents-install

# Development
dev:
	@echo "Starting Jura Trace development..."
	@echo "Run in separate terminals:"
	@echo "  Terminal 1: cd ui && npm run dev"
	@echo "  Terminal 2: cd src-tauri && cargo tauri dev"

# Install dependencies
install:
	cd ui && npm install
	cd src-tauri && cargo fetch

# Build production app
build:
	cd ui && npm run build
	cd src-tauri && cargo tauri build

# Type check
check:
	cd ui && npx svelte-check
	cd src-tauri && cargo check

# Run tests
test:
	cd src-tauri && cargo test
	cd ui && npm test

# Clean build artifacts
clean:
	cd ui && rm -rf node_modules .svelte-kit build
	cd src-tauri && cargo clean

# Format code
fmt:
	cd src-tauri && cargo fmt
	cd ui && npx prettier --write "src/**/*.{svelte,ts,js,css}"

# Lint
lint:
	cd src-tauri && cargo clippy
	cd ui && npx svelte-check

# Docker — local development stack (sidecar + Ollama)
docker-up:
	docker compose up -d

docker-down:
	docker compose down

docker-build:
	docker compose build

docker-logs:
	docker compose logs -f

# Advisory agents
agents-install:
	cd agents && pip install -r requirements.txt

agents:
	python -m agents --interactive
