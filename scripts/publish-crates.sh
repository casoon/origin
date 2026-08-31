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

# The 121 category slugs crates.io currently accepts (crates.io/api/v1/category_slugs,
# checked 2026-08-31). `cargo publish` rejects an unknown slug per crate, one at a time;
# checking here catches a typo before that.
valid_categories=(
  accessibility aerospace aerospace::drones aerospace::protocols aerospace::simulation
  aerospace::space-protocols aerospace::unmanned-aerial-vehicles algorithms api-bindings
  artificial-intelligence asynchronous authentication automotive caching
  command-line-interface command-line-utilities compilers compression computer-vision
  concurrency config cryptography cryptography::cryptocurrencies data-structures
  database database-implementations date-and-time development-tools
  development-tools::build-utils development-tools::cargo-plugins
  development-tools::debugging development-tools::ffi
  development-tools::procedural-macro-helpers development-tools::profiling
  development-tools::testing email embedded emulators encoding external-ffi-bindings
  filesystem finance game-development game-engines games graphics gui hardware-support
  internationalization localization mathematics memory-management multimedia
  multimedia::audio multimedia::encoding multimedia::images multimedia::video
  network-programming no-std no-std::no-alloc os os::android-apis os::freebsd-apis
  os::linux-apis os::macos-apis os::unix-apis os::windows-apis parser-implementations
  parsing rendering rendering::data-formats rendering::engine rendering::graphics-api
  rust-patterns science science::bioinformatics science::bioinformatics::genomics
  science::bioinformatics::proteomics science::bioinformatics::sequence-analysis
  science::computational-biology science::computational-biology::structural-modeling
  science::computational-biology::systems-biology science::computational-chemistry
  science::computational-chemistry::cheminformatics
  science::computational-chemistry::electronic-structure
  science::computational-chemistry::molecular-simulation science::geo science::materials
  science::neuroscience science::quantum-computing science::robotics security simulation
  template-engine text-editors text-processing value-formatting virtualization
  visualization wasm web-programming web-programming::http-client
  web-programming::http-server web-programming::websocket
)

# All 21 crates currently share the workspace version; if one is ever bumped on its own,
# switch its `wait_for_index`/`already_published` lookups to that crate's own version.
workspace_version="$(grep -m1 '^version = ' "$root/Cargo.toml" | sed -E 's/version = "(.*)"/\1/')"

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
check_names=false
for arg in "$@"; do
  case "$arg" in
    --execute) execute=true ;;
    --check-names) check_names=true ;;
    *)
      echo "usage: $0 [--execute] [--check-names]" >&2
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
    else
      for category in $(grep -m1 '^categories' "$manifest" | grep -oE '"[a-z0-9:-]+"' | tr -d '"'); do
        local known=false
        for valid in "${valid_categories[@]}"; do
          if [[ "$category" == "$valid" ]]; then
            known=true
            break
          fi
        done
        if [[ "$known" == false ]]; then
          echo "  - $name: \`$category\` is not a category crates.io recognises"
          failures=$((failures + 1))
        fi
      done
    fi
  done
  return "$failures"
}

# Best-effort: whether `name` is still unclaimed on crates.io. Network-dependent and
# never fatal on its own — a lookup failure (offline, blocked egress) is reported and
# skipped rather than treated as "taken" or aborting the whole check.
#
# Uses `cargo info`, not `curl`: some sandboxed/CI network policies allow cargo's own
# registry traffic while blocking arbitrary HTTPS clients, so `cargo info` is the more
# reliable check — and it is literally the tool `--execute` publishes with. Run from a
# scratch directory outside the workspace: inside it, every one of these 21 names
# resolves to its local path dependency instead of querying the registry.
check_names() {
  local taken=0
  local scratch
  scratch="$(mktemp -d)"

  for entry in "${crates[@]}"; do
    local name="${entry%%:*}"
    local output
    if output="$(cd "$scratch" && timeout 10 cargo info "$name" 2>&1)"; then
      echo "  - $name: already exists on crates.io"
      taken=$((taken + 1))
    elif echo "$output" | grep -qi "could not find\|does not exist"; then
      : # unclaimed
    else
      echo "  - $name: could not check — verify manually ($(echo "$output" | tail -1))"
    fi
  done

  rm -rf "$scratch"
  return "$taken"
}

wait_for_index() {
  local name="$1"
  for attempt in $(seq 1 30); do
    # The exact version, not just the crate name — `cargo info` succeeding on an
    # older already-indexed version would otherwise look like this publish landed.
    if cargo info "$name@$workspace_version" >/dev/null 2>&1; then
      return 0
    fi
    echo "  waiting for $name@$workspace_version to appear on crates.io ($attempt/30)..."
    sleep 10
  done
  echo "  $name@$workspace_version did not appear on the crates.io index in time" >&2
  return 1
}

already_published() {
  cargo info "$1@$workspace_version" >/dev/null 2>&1
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

if [[ "$check_names" == true ]]; then
  echo
  echo "checking name availability on crates.io..."
  if ! check_names; then
    echo
    echo "one or more names are already taken — see docs/publishing.md." >&2
    exit 1
  fi
  echo "names: all unclaimed"
fi

if [[ "$execute" == false ]]; then
  echo
  echo "dry run only; nothing was published. Re-run with --execute to publish for real, or"
  echo "with --check-names to check crates.io name availability."
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

  # Resumable: a crate already published at this version (from an earlier, partially
  # failed run) is skipped rather than failing the whole run on a re-publish attempt.
  if already_published "$name"; then
    echo "  $name@$workspace_version is already on crates.io — skipping"
    continue
  fi

  echo "\$ cargo publish --dry-run -p $name"
  cargo publish --dry-run -p "$name"

  echo "\$ cargo publish -p $name"
  cargo publish -p "$name"
  wait_for_index "$name"
done

echo
echo "all ${#crates[@]} crates published."
