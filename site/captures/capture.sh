#!/usr/bin/env bash
# Regenerates the terminal capture on the start page: the real output of scaffolding a
# new application from the template.
#   site/captures/capture.sh
# It creates ../my-app next to the repository and removes it again. Rebuild the SVG
# after a capture:
#   cd site/captures && npx -y -p @casoon/castwright@0.5.0 -p shiki castwright build new-app.terminal.yaml --format svg -o .
set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo"
if [ -e ../my-app ]; then
  echo "../my-app already exists" >&2
  exit 1
fi
cargo xtask new my-app --into .. 2>/dev/null > site/captures/new-app.txt
rm -rf ../my-app
