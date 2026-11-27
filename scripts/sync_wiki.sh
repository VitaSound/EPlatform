#!/usr/bin/env bash
# Regenerate reference pages, then publish docs/user → GitHub Wiki.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/docs/user"
WIKI_REMOTE="${WIKI_REMOTE:-git@github.com:VitaSound/EPlatform.wiki.git}"
WORKDIR="${WIKI_WORKDIR:-$(mktemp -d /tmp/eplatform-wiki-XXXXXX)}"
CLEANUP=1
if [[ "${WIKI_WORKDIR:-}" != "" ]]; then
  CLEANUP=0
fi

PAGES=(
  Home.md
  Quick-Start.md
  Architecture.md
  Rendering-Pipeline.md
  Paint-Commands.md
  Supported-Widgets.md
  Lua-Form-API.md
  Lua-App-API.md
  Host-Lua-API.md
  _Sidebar.md
)

echo "==> generate docs"
python3 "$ROOT/scripts/gen_user_docs.py" --out "$SRC"

if [[ ! -d "$SRC" ]]; then
  echo "missing $SRC" >&2
  exit 1
fi

echo "==> wiki workdir: $WORKDIR"
if [[ ! -d "$WORKDIR/.git" ]]; then
  git clone "$WIKI_REMOTE" "$WORKDIR"
else
  git -C "$WORKDIR" fetch origin
  git -C "$WORKDIR" checkout -B master origin/master 2>/dev/null \
    || git -C "$WORKDIR" checkout -B main origin/main
fi

for f in "${PAGES[@]}"; do
  if [[ ! -f "$SRC/$f" ]]; then
    echo "missing page: $SRC/$f" >&2
    exit 1
  fi
  cp "$SRC/$f" "$WORKDIR/$f"
done

rm -f "$WORKDIR/README.md" "$WORKDIR/gen_notes.yaml"

git -C "$WORKDIR" add -A
if git -C "$WORKDIR" diff --cached --quiet; then
  echo "wiki already up to date"
else
  git -C "$WORKDIR" -c user.email="${GIT_AUTHOR_EMAIL:-wiki-bot@users.noreply.github.com}" \
    -c user.name="${GIT_AUTHOR_NAME:-EPlatform docs}" \
    commit -m "Sync user docs from docs/user"
  git -C "$WORKDIR" push origin HEAD
  echo "wiki pushed"
fi

if [[ "$CLEANUP" -eq 1 ]]; then
  rm -rf "$WORKDIR"
fi
