# Hashed locks for CI tools

Each `.txt` here is compiled from the `.in` beside it with hashes, so CI can
install it with `pip install --require-hashes` (OpenSSF Scorecard,
Pinned-Dependencies).

| Lock | Used by |
|---|---|
| `ruff.txt` | `ci.yml`, Python job, lint and format |
| `pip-audit.txt` | `ci.yml`, Python job, in its own venv |
| `pyinstaller.txt` | `release.yml`, sidecar build (constrained by `sidecar/requirements.lock`) |
| `pip-licenses.txt` | `scripts/generate-licenses.sh` (constrained by `sidecar/requirements.lock`) |
| `cyclonedx-bom.txt` | `release.yml`, `sbom` job, the sidecar SBOM, in its own venv |

To change a version, edit the `.in` file, then from this directory run:

```
uv pip compile --universal --python-version 3.12 --generate-hashes --output-file=ruff.txt ruff.in
uv pip compile --universal --python-version 3.12 --generate-hashes --output-file=pip-audit.txt pip-audit.in
uv pip compile --universal --python-version 3.12 --generate-hashes -c ../../sidecar/requirements.lock --output-file=pyinstaller.txt pyinstaller.in
uv pip compile --universal --python-version 3.12 --generate-hashes -c ../../sidecar/requirements.lock --output-file=pip-licenses.txt pip-licenses.in
uv pip compile --universal --python-version 3.12 --generate-hashes --output-file=cyclonedx-bom.txt cyclonedx-bom.in
```

Recompile `pyinstaller.txt` and `pip-licenses.txt` whenever
`sidecar/requirements.lock` changes, so they stay inside its pins.
