#!/usr/bin/env python3
"""Serve client/crates/app/dist without browser caching.

`python3 -m http.server` leaves Chrome free to reuse a stale index.html/wasm
after a rebuild; this wrapper sends no-store for HTML and immutable caching
for Trunk's content-hashed assets.
"""

from __future__ import annotations

import argparse
import os
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DIR = ROOT / "crates" / "app" / "dist"


class Handler(SimpleHTTPRequestHandler):
    def end_headers(self) -> None:
        path = self.path.split("?", 1)[0]
        if path.endswith(".html") or path in ("", "/"):
            self.send_header("Cache-Control", "no-store, must-revalidate")
            self.send_header("Pragma", "no-cache")
            self.send_header("Expires", "0")
        elif path.endswith((".js", ".wasm")):
            self.send_header("Cache-Control", "public, max-age=31536000, immutable")
        else:
            self.send_header("Cache-Control", "no-cache")
        super().end_headers()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=8080)
    parser.add_argument("--bind", default="0.0.0.0")
    parser.add_argument("--dir", type=Path, default=DEFAULT_DIR)
    args = parser.parse_args()

    os.chdir(args.dir)
    server = ThreadingHTTPServer((args.bind, args.port), Handler)
    print(f"serving {args.dir} on http://{args.bind}:{args.port} (no-cache)")
    server.serve_forever()


if __name__ == "__main__":
    main()
