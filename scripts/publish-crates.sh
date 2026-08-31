#!/usr/bin/env bash
#
# Publish Origin's crates to crates.io, in dependency order.
#
# This is repository-local tooling, not part of `origin-xtask`: the tasks in that
# crate ship into every derivative's `cargo xtask`, and "publish Origin's own crates"
# has no meaning there.
#
# Usage:
#   scripts/publish-crates.sh              check crates.io metadata, publish nothing
#   scripts/publish-crates.sh --execute     actually publish, one crate at a time
#
# See docs/publishing.md for the full picture (why this order, what is deliberately
# excluded, and what to do if a publish fails partway through).

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Every crate a `--released` scaffold needs, in the order it must land on crates.io:
# each entry may depend on any before it, never on one after it (see
# crates/origin-xtask/src/scaffold.rs and docs/publishing.md for how that order was
# derived). `origin-ai`, `origin-mcp`, `origin-auth-loopback` and `origin-mcp-stdio`
# are deliberately absent: nothing in this list depends on them yet.
crates=(
  "origin-domain:crates"
  "origin-manifest:crates"
  "origin-events:crates"
  "origin-platform:crates"
  "origin-secrets:crates"
  "origin-storage:crates"
  "origin-http:crates"
  "origin-telemetry:crates"
  "origin-connector:crates"
  "origin-settings:crates"
  "origin-auth:crates"
  "origin-http-reqwest:adapters"
  "origin-secrets-system:adapters"
  "origin-storage-sqlite:adapters"
  "origin-notifications-tauri:adapters"
  "origin-jobs:crates"
  "origin-sync:crates"
  "origin-accounts:crates"
  "origin-app:crates"
  "origin-tauri:host"
  "origin-xtask:crates"
)

execute=false
for arg in "$@"; do
  case "$arg" in
    --execute) execute=true ;;
    *)
      echo "usage: $0 [--execute]" >&2
      exit 1
      ;;
  esac
done

check_metadata() {
  local failures=0
  for entry in "${crates[@]}"; do
    local name="${entry%%:*}"
    local layer="${entry##*:}"
    local manifest="$root/$layer/$name/Cargo.toml"

    if [[ ! -f "$manifest" ]]; then
      echo "  - $name: no manifest at $layer/$name/Cargo.toml"
      failures=$((failures + 1))
      continue
    fi
    if grep -qE '^publish\s*=\s*false' "$manifest"; then
      echo "  - $name: publish = false"
      failures=$((failures + 1))
    fi
    if ! grep -qE '^description\s*=' "$manifest"; then
      echo "  - $name: missing description"
      failures=$((failures + 1))
    fi
    if ! grep -qE '^license(\.workspace)?\s*=' "$manifest"; then
      echo "  - $name: missing license"
      failures=$((failures + 1))
    fi
    if ! grep -qE '^version(\.workspace)?\s*=' "$manifest"; then
      echo "  - $name: missing version"
      failures=$((failures + 1))
    fi
    if ! grep -qE '^readme\s*=' "$manifest"; then
      echo "  - $name: missing readme"
      failures=$((failures + 1))
    elif [[ ! -f "$root/$layer/$name/README.md" ]]; then
      echo "  - $name: readme is set but README.md is missing"
      failures=$((failures + 1))
    fi
    if ! grep -qE '^keywords\s*=' "$manifest"; then
      echo "  - $name: missing keywords"
      failures=$((failures + 1))
    fi
    if ! grep -qE '^categories\s*=' "$manifest"; then
      echo "  - $name: missing categories"
      failures=$((failures + 1))
    fi
  done
  return "$failures"
}

wait_for_index() {
  local name="$1"
  for attempt in $(seq 1 30); do
    if cargo info "$name" >/dev/null 2>&1; then
      return 0
    fi
    echo "  waiting for $name to appear on crates.io ($attempt/30)..."
    sleep 10
  done
  echo "  $name did not appear on the crates.io index in time" >&2
  return 1
}

echo "publish order (${#crates[@]} crates):"
for entry in "${crates[@]}"; do
  echo "  - ${entry%%:*}"
done
echo

echo "checking crates.io metadata..."
if ! check_metadata; then
  echo
  echo "metadata is incomplete; fix the issues above before publishing." >&2
  exit 1
fi
echo "metadata: ok"

if [[ "$execute" == false ]]; then
  echo
  echo "dry run only; nothing was published. Re-run with --execute to publish for real."
  exit 0
fi

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]] && ! grep -q "token" "${CARGO_HOME:-$HOME/.cargo}/credentials.toml" 2>/dev/null; then
  echo "no crates.io credentials found (CARGO_REGISTRY_TOKEN unset, no ~/.cargo/credentials.toml)." >&2
  echo "run \`cargo login\` first." >&2
  exit 1
fi

for entry in "${crates[@]}"; do
  name="${entry%%:*}"
  echo
  echo "=== $name ==="
  echo "\$ cargo publish -p $name"
  cargo publish -p "$name"
  wait_for_index "$name"
done

echo
echo "all ${#crates[@]} crates published."
