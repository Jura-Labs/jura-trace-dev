# Jura Trace — Python ML Sidecar
# Base: python:3.12-slim (glibc needed for opencv/numpy wheels)

# Pinned by digest (OpenSSF Scorecard, Pinned-Dependencies); the tag is kept
# for readers. Dependabot's docker updates move both together.
FROM python:3.12-slim@sha256:aa943ce411423c7582288f18707ff66eddcbaeb510502d747a72aeb34f30d159 AS base

# System deps for opencv-python-headless
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libgl1 \
        libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for security
RUN groupadd --gid 1000 jura \
    && useradd --uid 1000 --gid jura --create-home jura

WORKDIR /app

# Install Python deps first (cache-friendly layer ordering)
# The hashed lock, as CI and release builds use (OpenSSF Scorecard,
# Pinned-Dependencies).
COPY requirements.lock .
RUN pip install --no-cache-dir --require-hashes -r requirements.lock

# Copy application code (changes most frequently)
COPY main.py .
COPY app/ app/

# Switch to non-root user
USER jura

EXPOSE 8200

# Health check using Python urllib (no curl in slim image)
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8200/health')" || exit 1

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8200"]
