# SPDX-License-Identifier: AGPL-3.0-or-later

"""HEIC/HEIF codec availability tests.

The sidecar's lifespan handler imports `pillow_heif` and registers it
with PIL so iPhone photos (.heic / .heif) decode through `Image.open()`.
On macOS development boxes, system ImageIO frequently masks a missing
pillow-heif install; on Linux PyInstaller bundles it does not, and HEIC
files silently fail to decode.

These tests pin the contract:

* `register_heif_opener()` must succeed at import time (positive case).
* If pillow-heif is missing, the lifespan handler must log CRITICAL —
  but not raise — so that other formats still verify.
"""

import importlib
import logging

import pytest


def test_pillow_heif_register_opener_succeeds():
    """pillow-heif must be importable and register without error."""
    import pillow_heif

    pillow_heif.register_heif_opener()

    from PIL import Image

    assert (
        "HEIF" in Image.registered_extensions().get(".heic", "HEIF")
        or ".heic" in Image.registered_extensions()
    ), "pillow-heif did not register the .heic extension with PIL"


@pytest.mark.asyncio
async def test_lifespan_logs_critical_when_pillow_heif_missing(
    caplog: pytest.LogCaptureFixture, monkeypatch: pytest.MonkeyPatch
):
    """If pillow-heif is unavailable, the lifespan handler must log
    CRITICAL and continue (other formats remain functional)."""
    import builtins

    original_import = builtins.__import__

    def fake_import(name: str, *args: object, **kwargs: object):
        if name == "pillow_heif":
            raise ImportError("simulated missing pillow-heif")
        return original_import(name, *args, **kwargs)

    monkeypatch.setattr(builtins, "__import__", fake_import)

    import main as sidecar_main

    importlib.reload(sidecar_main)

    caplog.set_level(logging.CRITICAL, logger=sidecar_main.logger.name)

    async with sidecar_main.lifespan(sidecar_main.app):
        pass

    critical_messages = [
        r.message
        for r in caplog.records
        if r.levelno == logging.CRITICAL and "pillow-heif" in r.message
    ]
    assert critical_messages, (
        "Expected a CRITICAL log mentioning pillow-heif when the import "
        f"is monkey-patched to fail. Captured: {[r.message for r in caplog.records]}"
    )

    monkeypatch.undo()
    importlib.reload(sidecar_main)
