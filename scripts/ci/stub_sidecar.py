#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""A sidecar that looks healthy and analyses nothing, for assertion A4
(docs/design/v1.2.0-headless-api-and-cli.md, section 11).

It answers /health and /health/ready with 200, so the server and its
supervisor treat it as up, and every other route with 503, so no detector
produces a result. scripts/ci/cli-against-server.sh runs a verification
through it and requires scripts/assert_sidecar_ran.py to FAIL: the only way
to know that check can fail is to watch it fail.

Started by the server's supervisor as `<this> --host 127.0.0.1 --port N`.
Writes one line per request to $STUB_SIDECAR_LOG, so the caller can prove
the pipeline really called it.
"""

import json
import os
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LOG = os.environ.get("STUB_SIDECAR_LOG")


class Handler(BaseHTTPRequestHandler):
    def _answer(self):
        length = int(self.headers.get("Content-Length") or 0)
        if length:
            self.rfile.read(length)
        path = self.path.split("?")[0]
        if LOG:
            with open(LOG, "a") as f:
                f.write(f"{self.command} {path}\n")
        if path in ("/health", "/health/ready"):
            status, body = 200, {"status": "ok", "version": "stub", "capabilities": {}}
        else:
            status, body = 503, {"detail": "stub sidecar: analysis not available"}
        payload = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    do_GET = _answer
    do_POST = _answer

    def log_message(self, *args):
        pass


def main():
    args = sys.argv[1:]
    host = args[args.index("--host") + 1] if "--host" in args else "127.0.0.1"
    port = int(args[args.index("--port") + 1])
    ThreadingHTTPServer((host, port), Handler).serve_forever()


if __name__ == "__main__":
    main()
