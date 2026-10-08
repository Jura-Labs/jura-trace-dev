# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Every test runs with a configured sidecar key.

The auth middleware refuses /forensics requests when no key is set, with no
exception for the test runner (JTV-205), so tests that go through the real
app send AUTH_HEADERS. monkeypatch restores the original value afterwards.
"""

import pytest

from app.config import settings
from tests.sidecar_auth import TEST_SIDECAR_KEY


@pytest.fixture(autouse=True)
def sidecar_key(monkeypatch):
    monkeypatch.setattr(settings, "sidecar_key", TEST_SIDECAR_KEY)
