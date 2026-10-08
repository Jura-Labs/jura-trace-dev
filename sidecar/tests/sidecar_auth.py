# SPDX-License-Identifier: AGPL-3.0-or-later

"""The sidecar key every test runs with. conftest.py configures it."""

TEST_SIDECAR_KEY = "test-sidecar-key"
AUTH_HEADERS = {"X-Jura-API-Key": TEST_SIDECAR_KEY}
