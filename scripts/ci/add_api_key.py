#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Create an API key in an installed Jura Trace database, for CI only.

The installers do not ship jura-trace-api yet (stage 5), so a test that
drives an installed app through the REST API has no supported way to get a
key. This writes one the way the server does (src-tauri/src/api/routes.rs,
create_api_key): a random raw key, stored as the SHA-256 hex of the raw key
without its jt_ prefix, and printed once as jt_<raw>.

Usage: add_api_key.py <path to jura_trace.db> [--name ci]
"""

import argparse
import datetime
import hashlib
import secrets
import sqlite3
import sys
import uuid


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("db")
    ap.add_argument("--name", default="ci")
    args = ap.parse_args()
    raw = secrets.token_hex(32)
    con = sqlite3.connect(args.db, timeout=10)
    try:
        con.execute(
            "INSERT INTO api_keys (key_id, name, key_hash, rate_limit, revoked, created_at) "
            "VALUES (?, ?, ?, ?, 0, ?)",
            (
                str(uuid.uuid4()),
                args.name,
                hashlib.sha256(raw.encode()).hexdigest(),
                600,
                datetime.datetime.now(datetime.timezone.utc).isoformat(),
            ),
        )
        con.commit()
    except sqlite3.Error as e:
        print(f"add_api_key: {args.db}: {e}", file=sys.stderr)
        return 1
    finally:
        con.close()
    print(f"jt_{raw}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
