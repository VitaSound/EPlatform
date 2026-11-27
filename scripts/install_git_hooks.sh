#!/usr/bin/env bash
# Point this clone at tracked hooks in .githooks/
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit
echo "hooksPath → .githooks (pre-commit will regen docs/user when sources are staged)"
