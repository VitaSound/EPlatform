#!/usr/bin/env python3
"""Serve client/crates/app/dist without browser caching.

`python3 -m http.server` leaves Chrome free to reuse a stale index.html/wasm
after a rebuild; this wrapper sends no-store for HTML and immutable caching
for Trunk's content-hashed assets.

It also gzips large text/binary assets on the fly. The wasm bundle is ~10 MB
raw and ~3.4 MB gzipped, which matters a lot over a phone/WireGuard link:
`http.server` never compresses, so a mobile client used to download the full
10 MB before the app could even start.
"""

from __future__ import annotations

import argparse
import gzip
import os
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DIR = ROOT / "crates" / "app" / "dist"

# Suffixes worth compressing (wasm/js/html make up the whole payload).
COMPRESSIBLE = {".wasm", ".js", ".mjs", ".html", ".css", ".json", ".svg", ".xml"}
# Assets smaller than this grow when framed, so skip them.
MIN_COMPRESS = 1024

_gzip_cache: dict[str, tuple[float, bytes]] = {}
_gzip_lock = threading.Lock()


def gzipped(path: Path) -> bytes:
    """Return the gzip-compressed payload for `path`, cached by mtime."""
    stamp = path.stat().st_mtime
    key = str(path)
    with _gzip_lock:
        cached = _gzip_cache.get(key)
        if cached is not None and cached[0] == stamp:
            return cached[1]
    data = gzip.compress(path.read_bytes(), 6)
    with _gzip_lock:
        _gzip_cache[key] = (stamp, data)
    return data


class Handler(SimpleHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

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

    def send_head(self):
        """Fall back to the plain file except for compressible, sizeable assets."""
        target = Path(self.translate_path(self.path))
        wants_gzip = "gzip" in self.headers.get("Accept-Encoding", "").lower()
        if (
            wants_gzip
            and target.is_file()
            and target.suffix.lower() in COMPRESSIBLE
            and target.stat().st_size >= MIN_COMPRESS
        ):
            body = gzipped(target)
            self.send_response(200)
            self.send_header("Content-Type", self.guess_type(str(target)))
            self.send_header("Content-Encoding", "gzip")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Vary", "Accept-Encoding")
            self.send_header("Last-Modified", self.date_time_string(target.stat().st_mtime))
            self.end_headers()
            return _Bytes(body)
        return super().send_head()


class _Bytes:
    """Minimal file-like wrapper so `copyfile` can stream an in-memory body."""

    def __init__(self, data: bytes) -> None:
        self._data = data
        self._pos = 0

    def read(self, amount: int = -1) -> bytes:
        if amount is None or amount < 0:
            chunk, self._pos = self._data[self._pos:], len(self._data)
            return chunk
        chunk = self._data[self._pos:self._pos + amount]
        self._pos += len(chunk)
        return chunk

    def close(self) -> None:
        pass


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=8080)
    parser.add_argument("--bind", default="0.0.0.0")
    parser.add_argument("--dir", type=Path, default=DEFAULT_DIR)
    args = parser.parse_args()

    os.chdir(args.dir)
    server = ThreadingHTTPServer((args.bind, args.port), Handler)
    print(f"serving {args.dir} on http://{args.bind}:{args.port} (no-cache, gzip)")
    server.serve_forever()


if __name__ == "__main__":
    main()
