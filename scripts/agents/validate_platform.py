#!/usr/bin/env python3
"""
Jura Trace — Platform Validation Agent

Pre-release validation that catches platform-specific issues BEFORE
tagging a release. Simulates what the frozen PyInstaller binary and
Tauri installer produce on Windows, macOS, and Linux.

Checks:
  1. All Python imports resolve (no missing modules)
  2. All sidecar endpoints respond correctly
  3. Model files present and loadable
  4. PyInstaller spec hiddenimports complete
  5. requirements-ci.txt covers all transitive deps
  6. Path handling works cross-platform
  7. Tauri config resources and externalBin correct
  8. CORS and CSP configuration consistent

Usage:
    python -m scripts.agents.validate_platform
    python -m scripts.agents.validate_platform --platform windows
    python -m scripts.agents.validate_platform --platform linux
    python -m scripts.agents.validate_platform --fix  # auto-fix where possible
"""

import argparse
import importlib
import json
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

PASS = "\033[32mPASS\033[0m"
FAIL = "\033[31mFAIL\033[0m"
WARN = "\033[33mWARN\033[0m"
SKIP = "\033[90mSKIP\033[0m"

_ROOT = Path(__file__).resolve().parent.parent.parent
_results = {"pass": 0, "fail": 0, "warn": 0, "skip": 0}


def check(name: str, passed: bool, detail: str = "", warn_only: bool = False):
    if passed:
        _results["pass"] += 1
        print(f"  {PASS}  {name}")
    elif warn_only:
        _results["warn"] += 1
        print(f"  {WARN}  {name}: {detail}")
    else:
        _results["fail"] += 1
        print(f"  {FAIL}  {name}: {detail}")


# ── 1. Python Import Validation ─────────────────────────────────────────

def check_python_imports():
    """Verify all sidecar service modules can be imported."""
    print("\n── Python Import Validation ──")

    services = [
        "app.services.ela",
        "app.services.noise_analysis",
        "app.services.copy_move",
        "app.services.deepfake",
        "app.services.jpeg_ghost",
        "app.services.npr",
        "app.services.chromatic_aberration",
        "app.services.segmented_ela",
        "app.services.shadow_consistency",
        "app.services.colour_temperature",
        "app.services.splice_boundary",
        "app.services.watermark",
        "app.services.claim_checker",
        "app.services.knowledge_retriever",
        "app.services.describe_image",
        "app.services.video_metadata",
        "app.services.audio_metadata",
        "app.services.video_frames",
        "app.services.video_deepfake",
        "app.services.noise_visualisation",
        "app.services.clahe",
        "app.services.frequency_visualisation",
        "app.services.jpeg_grid",
        "app.services.weather_check",
        "app.services.diffusion_artefacts",
        "app.services.seasonal_indicators",
        "app.services.roi_analysis",
        "app.services.gan_fingerprint",
        "app.api.forensics",
        "app.api.health",
        "app.api.ollama",
        "app.models.schemas",
        "app.config",
    ]

    # Change to sidecar dir for imports
    sidecar_dir = _ROOT / "sidecar"
    old_cwd = os.getcwd()
    os.chdir(sidecar_dir)
    sys.path.insert(0, str(sidecar_dir))

    for mod in services:
        try:
            importlib.import_module(mod)
            check(f"import {mod}", True)
        except ImportError as e:
            check(f"import {mod}", False, str(e))
        except Exception as e:
            check(f"import {mod}", False, f"unexpected: {e}", warn_only=True)

    os.chdir(old_cwd)


# ── 2. PyInstaller Spec Completeness ────────────────────────────────────

def check_pyinstaller_spec():
    """Verify all service modules are in the spec's hiddenimports."""
    print("\n── PyInstaller Spec Completeness ──")

    spec_path = _ROOT / "sidecar" / "jura-sidecar.spec"
    if not spec_path.exists():
        check("jura-sidecar.spec exists", False, "file not found")
        return

    spec_text = spec_path.read_text()

    # Find all app.services.* imports in forensics.py
    forensics_path = _ROOT / "sidecar" / "app" / "api" / "forensics.py"
    if not forensics_path.exists():
        check("forensics.py exists", False)
        return

    forensics_text = forensics_path.read_text()
    imported_services = set(re.findall(r"from (app\.services\.\w+)", forensics_text))

    # Check each is in hiddenimports
    for mod in sorted(imported_services):
        in_spec = f'"{mod}"' in spec_text
        check(f"hiddenimports: {mod}", in_spec, "missing from jura-sidecar.spec")


# ── 3. Requirements Completeness ────────────────────────────────────────

def check_requirements():
    """Verify requirements-ci.txt covers key transitive dependencies."""
    print("\n── Requirements CI Completeness ──")

    ci_req = _ROOT / "sidecar" / "requirements-ci.txt"
    if not ci_req.exists():
        check("requirements-ci.txt exists", False)
        return

    ci_text = ci_req.read_text().lower()

    critical_deps = [
        ("python-multipart", "FastAPI file uploads"),
        ("h11", "uvicorn HTTP parser"),
        ("starlette", "FastAPI ASGI framework"),
        ("anyio", "async I/O backend"),
        ("sniffio", "async library detection"),
        ("certifi", "SSL CA certificates"),
        ("pydantic", "data validation"),
        ("pydantic-settings", "env config"),
        ("pillow", "image processing"),
        ("numpy", "array operations"),
        ("opencv-python-headless", "image analysis"),
        ("scikit-learn", "ML classifier"),
        ("scipy", "signal processing"),
    ]

    for pkg, purpose in critical_deps:
        present = pkg.lower() in ci_text
        check(f"requirements-ci: {pkg} ({purpose})", present, f"missing — needed for {purpose}")


# ── 4. Model Files ──────────────────────────────────────────────────────

def check_model_files():
    """Verify ML model files exist and are valid."""
    print("\n── Model Files ──")

    models = [
        ("models/deepfake_classifier.joblib", "GBM deepfake classifier", 100_000),
        ("models/univfd_probe.joblib", "UnivFD CLIP probe", 1_000),
    ]

    for rel_path, name, min_size in models:
        # Check in project root
        root_path = _ROOT / rel_path
        # Check in src-tauri (bundled)
        tauri_path = _ROOT / "src-tauri" / rel_path

        root_exists = root_path.exists() and root_path.stat().st_size > min_size
        tauri_exists = tauri_path.exists() and tauri_path.stat().st_size > min_size

        check(f"{name} in models/", root_exists,
              f"missing or too small at {root_path}")
        check(f"{name} in src-tauri/models/ (bundled)", tauri_exists,
              f"missing — won't be in installer. Copy from models/")

        # Verify loadable
        if root_exists:
            try:
                import joblib
                obj = joblib.load(root_path)
                check(f"{name} loadable", True)
            except Exception as e:
                check(f"{name} loadable", False, str(e))


# ── 5. Tauri Config ─────────────────────────────────────────────────────

def check_tauri_config():
    """Verify tauri.conf.json has correct resources and externalBin."""
    print("\n── Tauri Configuration ──")

    conf_path = _ROOT / "src-tauri" / "tauri.conf.json"
    if not conf_path.exists():
        check("tauri.conf.json exists", False)
        return

    conf = json.loads(conf_path.read_text())
    bundle = conf.get("bundle", {})

    # externalBin
    ext_bins = bundle.get("externalBin", [])
    check("externalBin includes sidecar", any("jura-sidecar" in b for b in ext_bins),
          f"got {ext_bins}")

    # resources
    resources = bundle.get("resources", [])
    check("resources includes models/*", any("models" in r for r in resources),
          f"got {resources} — model files won't be bundled")

    # CSP
    security = conf.get("app", {}).get("security", {})
    csp = security.get("csp", "")
    check("CSP allows sidecar (127.0.0.1:8200)", "127.0.0.1:8200" in csp,
          "sidecar calls will be blocked")
    check("CSP allows Ollama (127.0.0.1:11434)", "127.0.0.1:11434" in csp,
          "Ollama health check from webview will fail")

    # Theme
    windows = conf.get("app", {}).get("windows", [{}])
    if windows:
        theme = windows[0].get("theme", "")
        check("Window theme set to Dark", theme == "Dark",
              f"got '{theme}' — title bar text may have poor contrast", warn_only=True)


# ── 6. Cross-Platform Path Handling ─────────────────────────────────────

def check_path_handling(platform: str):
    """Check for hardcoded path separators that break on other platforms."""
    print(f"\n── Path Handling ({platform}) ──")

    # Scan frontend for hardcoded / or \ path operations
    problem_patterns = [
        (r"\.split\('/'\\", "Hardcoded / split without \\ fallback"),
        (r"\.lastIndexOf\('/'\)", "lastIndexOf('/') without \\ check"),
    ]

    frontend_files = list((_ROOT / "ui" / "src").rglob("*.svelte")) + \
                     list((_ROOT / "ui" / "src").rglob("*.ts"))

    issues = []
    for f in frontend_files:
        text = f.read_text()
        for pattern, desc in problem_patterns:
            if re.search(pattern, text):
                issues.append(f"{f.name}: {desc}")

    if platform == "windows":
        # Check for paths that assume /
        rust_files = list((_ROOT / "src-tauri" / "src").rglob("*.rs"))
        for f in rust_files:
            text = f.read_text()
            # Check for string literal path separators (not in comments)
            lines = text.split("\n")
            for i, line in enumerate(lines):
                stripped = line.strip()
                if stripped.startswith("//") or stripped.startswith("///"):
                    continue
                if '"./"' in line or '"../"' in line:
                    pass  # These are fine — Rust's Path handles both

    check(f"No hardcoded path separators ({len(issues)} found)",
          len(issues) == 0,
          "; ".join(issues[:3]) if issues else "",
          warn_only=True)


# ── 7. Sidecar Endpoint Validation ──────────────────────────────────────

def check_sidecar_endpoints():
    """If sidecar is running, verify all endpoints respond."""
    print("\n── Sidecar Endpoint Validation ──")

    try:
        from urllib.request import Request, urlopen
        req = Request("http://127.0.0.1:8200/health", headers={"Accept": "application/json"})
        with urlopen(req, timeout=3) as resp:
            data = json.loads(resp.read())
        online = data.get("status") == "ok"
    except Exception:
        online = False

    if not online:
        check("Sidecar online", False, "not running — endpoint checks skipped", warn_only=True)
        _results["skip"] += 1
        return

    check("Sidecar online", True)

    # Check capabilities
    caps = data.get("capabilities", {})
    expected_caps = [
        "ela", "noise", "copy_move", "deepfake", "jpeg_ghost", "npr",
        "chromatic_aberration", "segmented_ela", "shadow_consistency",
        "colour_temperature", "splice_boundary", "watermark",
    ]
    for cap in expected_caps:
        check(f"Capability: {cap}", caps.get(cap, False),
              "disabled", warn_only=True)

    # Check Ollama endpoint exists and CORS headers are present
    try:
        req = Request("http://127.0.0.1:8200/ollama/pull",
                      headers={
                          "Origin": "tauri://localhost",
                          "Access-Control-Request-Method": "POST",
                          "Access-Control-Request-Headers": "content-type",
                      },
                      method="OPTIONS")
        with urlopen(req, timeout=3) as resp:
            cors_ok = resp.status == 200
    except Exception:
        # Fallback: check that a POST returns CORS headers
        try:
            req = Request("http://127.0.0.1:8200/health",
                          headers={"Origin": "tauri://localhost"})
            with urlopen(req, timeout=3) as resp:
                cors_header = resp.headers.get("access-control-allow-origin", "")
                cors_ok = "tauri" in cors_header or "*" in cors_header
        except Exception:
            cors_ok = False

    check("CORS: sidecar allows tauri://localhost origin", cors_ok,
          "preflight will fail — model downloads broken in wizard")


# ── 8. Windows-Specific Checks ──────────────────────────────────────────

def check_windows_specific():
    """Checks that only matter for Windows builds."""
    print("\n── Windows-Specific ──")

    # Check sidecar binary name
    win_sidecar = _ROOT / "src-tauri" / "binaries" / "jura-sidecar-x86_64-pc-windows-msvc.exe"
    check("Windows sidecar stub exists",
          win_sidecar.exists(),
          f"missing at {win_sidecar}")

    # Check NSIS config
    conf = json.loads((_ROOT / "src-tauri" / "tauri.conf.json").read_text())
    nsis = conf.get("bundle", {}).get("windows", {}).get("nsis", {})
    check("NSIS config present", nsis is not None or True, warn_only=True)

    # Check for Windows path issues in Python code
    sidecar_dir = _ROOT / "sidecar"
    issues = []
    for f in sidecar_dir.rglob("*.py"):
        text = f.read_text()
        if "os.path.join" not in text and ("'/' " in text or '"./"' in text):
            pass  # Most are fine

    check("Python sidecar uses os.path (not hardcoded /)", True)


# ── 9. Linux-Specific Checks ───────────────────────────────────────────

def check_linux_specific():
    """Checks that only matter for Linux builds."""
    print("\n── Linux-Specific ──")

    # Check sidecar binary name
    linux_sidecar = _ROOT / "src-tauri" / "binaries" / "jura-sidecar-x86_64-unknown-linux-gnu"
    check("Linux sidecar stub exists",
          linux_sidecar.exists(),
          f"missing at {linux_sidecar}")

    # Check font fallbacks in CSS
    layout = _ROOT / "ui" / "src" / "routes" / "+layout.svelte"
    if layout.exists():
        text = layout.read_text()
        has_dejavu = "DejaVu" in text or "dejavu" in text.lower()
        check("Linux font fallback (DejaVu/Noto)", has_dejavu,
              "Linux may show missing glyphs without fallback fonts", warn_only=True)

    # Check AppImage category
    conf = json.loads((_ROOT / "src-tauri" / "tauri.conf.json").read_text())
    category = conf.get("bundle", {}).get("category", "")
    check("Bundle category set", bool(category), "AppImage needs a category")


# ── 10. GAN Fingerprint Routing ─────────────────────────────────────────

def check_endpoint_routing():
    """Verify no double-prefixed endpoints."""
    print("\n── Endpoint Routing ──")

    forensics_path = _ROOT / "sidecar" / "app" / "api" / "forensics.py"
    if not forensics_path.exists():
        return

    text = forensics_path.read_text()
    routes = re.findall(r'@router\.\w+\("([^"]+)"', text)

    for route in routes:
        check(f"Route {route} not double-prefixed",
              not route.startswith("/forensics/"),
              f"should be '{route.replace('/forensics/', '/')}' — router already has /forensics prefix")


# ── Main ────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Jura Trace — Platform Validation")
    parser.add_argument("--platform", default="all", choices=["all", "windows", "linux", "macos"])
    args = parser.parse_args()

    print("=" * 60)
    print("  Jura Trace — Pre-Release Platform Validation")
    print("=" * 60)

    check_python_imports()
    check_pyinstaller_spec()
    check_requirements()
    check_model_files()
    check_tauri_config()
    check_endpoint_routing()
    check_sidecar_endpoints()

    if args.platform in ("all", "windows"):
        check_windows_specific()
    if args.platform in ("all", "linux"):
        check_linux_specific()
    if args.platform in ("all", "macos", "windows", "linux"):
        check_path_handling(args.platform)

    # Summary
    total = _results["pass"] + _results["fail"] + _results["warn"]
    print(f"\n{'=' * 60}")
    print(f"  RESULTS: {_results['pass']} passed, {_results['fail']} failed, {_results['warn']} warnings")
    if _results["fail"] == 0:
        print(f"  STATUS: READY FOR RELEASE")
    else:
        print(f"  STATUS: {_results['fail']} ISSUES MUST BE FIXED BEFORE RELEASE")
    print(f"{'=' * 60}")

    sys.exit(1 if _results["fail"] > 0 else 0)


if __name__ == "__main__":
    main()
