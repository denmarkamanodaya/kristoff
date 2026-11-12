#!/usr/bin/env bash
# Every Kristoff Desktop gate, in order (docs/desktop/phases/phase-1-core-storage.md §7). Exits
# non-zero on the first failure. Run before every push and paste the output in the PR.
# Usage: bash desktop/scripts/ci-local.sh
set -euo pipefail
cd "$(dirname "$0")/.."
# Homebrew rustup does not put its proxies on PATH; use the pinned toolchain directly.
export PATH="$HOME/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"
DRY_GATE="$HOME/Projects/.claude/skills/dk-coding/scripts/dry-gate.mjs"
GATE_LOG="$(mktemp -t kristoff-ci-gate)"
trap 'rm -f "$GATE_LOG"' EXIT

gate() {
  local name="$1"
  shift
  if "$@" >"$GATE_LOG" 2>&1; then
    echo "PASS $name"
  else
    echo "FAIL $name"
    tail -40 "$GATE_LOG"
    exit 1
  fi
}

skip() {
  echo "SKIP $1 ($2)"
}

# The 300-line cap counts lines that are not blank and not comments (stack/code-style.md).
rust_file_sizes() {
  local bad=0
  while IFS= read -r f; do
    local n
    n=$(grep -cvE '^\s*($|//)' "$f" || true)
    if [ "$n" -gt 300 ]; then
      echo "$f: $n lines (max 300)"
      bad=1
    fi
  done < <(find crates app/src-tauri/src app/src-tauri/tests -name '*.rs' -not -path '*/target/*')
  return "$bad"
}

# jscpd: pinned as a web devDependency (owner approved 2026-11-12); run `npm ci` in web/ first.
find_jscpd() {
  local candidate="../web/node_modules/.bin/jscpd"
  [ -x "$candidate" ] && echo "$candidate"
}

gate "cargo fmt" cargo fmt --all --check
gate "cargo clippy" cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic
gate "rust file size" rust_file_sizes
gate "cargo test" cargo test --workspace
gate "cargo deny" cargo deny check licenses advisories
gate "web check" npm --prefix ../web run -s check
JSCPD="$(find_jscpd || true)"
if [ -z "$JSCPD" ] || [ ! -f "$DRY_GATE" ]; then
  echo "FAIL dry-gate (rust)"
  echo "jscpd or $DRY_GATE not found; the DRY gate never skips"
  exit 1
fi
gate "dry-gate (rust)" env DRY_GATE_JSCPD="$JSCPD" node "$DRY_GATE" --root .
echo "ALL GATES PASSED"
