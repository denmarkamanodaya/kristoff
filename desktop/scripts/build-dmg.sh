#!/usr/bin/env bash
# Builds the release DMG for Kristoff Desktop (docs/desktop/phases/phase-5-eval-package.md §2):
# checks the two baked secrets (never printed), verifies the law index and the e5 model against
# their lock files, checks THIRD_PARTY_NOTICES, builds the desktop web bundle, builds the arm64
# app and DMG with the bundled resources, verifies the ad-hoc signature, and writes
# desktop/dist/Kristoff-<version>-arm64.dmg plus its .sha256.
#
# Usage: KRISTOFF_ANTHROPIC_KEY=... KRISTOFF_TOTP_SECRET=... bash desktop/scripts/build-dmg.sh [--dry-run]
#   --dry-run  runs every check and the web build, then stops before the release build (nothing
#              is compiled, so nothing is baked).
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:$HOME/.cargo/bin:$PATH"

DRY_RUN=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

TARGET="aarch64-apple-darwin"
RFC_TEST_SECRET="GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"
DATA="data"

step() { echo "==> $*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

check_secrets() {
  step "secrets (values are never printed)"
  local key="${KRISTOFF_ANTHROPIC_KEY:-}" totp="${KRISTOFF_TOTP_SECRET:-}"
  [ -n "$key" ] || fail "KRISTOFF_ANTHROPIC_KEY is not set"
  [ -n "$totp" ] || fail "KRISTOFF_TOTP_SECRET is not set"
  case "$key" in
    sk-ant-test*) fail "KRISTOFF_ANTHROPIC_KEY is the fake dev key" ;;
    sk-ant-*) ;;
    *) fail "KRISTOFF_ANTHROPIC_KEY does not look like a Claude API key (sk-ant-...)" ;;
  esac
  local norm
  norm="$(printf '%s' "$totp" | tr -d ' =' | tr '[:lower:]' '[:upper:]')"
  [ "$norm" != "$RFC_TEST_SECRET" ] || fail "KRISTOFF_TOTP_SECRET is the RFC 6238 test secret"
  printf '%s' "$norm" | grep -Eq '^[A-Z2-7]{16,}$' || fail "KRISTOFF_TOTP_SECRET is not base32 (16+ characters)"
  echo "ok: both set, neither is a dev value"
}

lock_value() { sed -n "s/^# $2: //p" "$DATA/$1"; }

check_model() {
  step "e5 model files against model.lock"
  [ -d "$DATA/model" ] || fail "$DATA/model is missing (run scripts/fetch-model.sh)"
  (cd "$DATA/model" && grep -E '^[0-9a-f]{64}  ' ../model.lock | shasum -a 256 -c -) \
    || fail "model files do not match model.lock"
}

check_dataset_sources() {
  if [ -d "$DATA/source" ]; then
    step "dataset parquet files against dataset.lock"
    (cd "$DATA/source" && grep -E '^[0-9a-f]{64}  ' ../dataset.lock | shasum -a 256 -c -) \
      || fail "dataset files do not match dataset.lock"
  fi
}

meta() { sqlite3 "$DATA/phlaws.db" "SELECT value FROM meta WHERE key = '$1'"; }

check_phlaws() {
  step "phlaws.db against the lock revisions"
  [ -s "$DATA/phlaws.db" ] || fail "$DATA/phlaws.db is missing (run kristoff-ingest)"
  [ "$(meta dataset_revision)" = "$(lock_value dataset.lock revision)" ] \
    || fail "phlaws.db was built from another dataset revision"
  [ "$(meta model_revision)" = "$(lock_value model.lock revision)" ] \
    || fail "phlaws.db was built with another model revision"
  [ "$(meta chunks_stage)" = "done" ] || fail "phlaws.db ingest did not finish"
  local chunks vectors
  chunks="$(meta chunk_count)"; vectors="$(meta vector_count)"
  [ -n "$chunks" ] && [ "$chunks" = "$vectors" ] || fail "phlaws.db has $vectors vectors for $chunks chunks"
  [ "$(sqlite3 "$DATA/phlaws.db" 'PRAGMA quick_check')" = "ok" ] || fail "phlaws.db fails quick_check"
  echo "ok: dataset $(meta dataset_revision | cut -c1-8), model $(meta model_revision | cut -c1-8), $chunks chunks"
}

app_version() { node -p "require('./app/src-tauri/tauri.conf.json').version"; }

release_build() {
  step "release build ($TARGET, app + dmg, bundled resources)"
  (cd app/src-tauri && cargo tauri build --target "$TARGET" --bundles app,dmg \
    --config tauri.bundle.conf.json)
}

verify_app() {
  local app="target/$TARGET/release/bundle/macos/Kristoff.app"
  step "codesign --verify --deep ($app)"
  codesign --verify --deep --strict "$app" || fail "the app signature does not verify"
  codesign -dv "$app" 2>&1 | grep -q 'Signature=adhoc' || fail "the app is not ad-hoc signed"
  for res in phlaws.db models/e5-small/model.safetensors THIRD_PARTY_NOTICES; do
    [ -s "$app/Contents/Resources/$res" ] || fail "the app lacks the resource $res"
  done
  echo "ok: ad-hoc signature verifies; resources present"
}

publish_dmg() {
  local version dmg out
  version="$(app_version)"
  dmg="$(ls -t target/$TARGET/release/bundle/dmg/*.dmg | head -1)"
  out="dist/Kristoff-$version-arm64.dmg"
  step "copy to $out"
  mkdir -p dist
  cp "$dmg" "$out"
  (cd dist && shasum -a 256 "$(basename "$out")" > "$(basename "$out").sha256")
  echo "size: $(du -h "$out" | cut -f1)"
  cat "$out.sha256"
}

check_secrets
check_model
check_dataset_sources
check_phlaws
step "THIRD_PARTY_NOTICES"
node scripts/third-party-notices.mjs --check
step "desktop web build"
npm --prefix ../web run -s build -- --target desktop
if [ "$DRY_RUN" = 1 ]; then
  step "dry run: stopping before the release build (version $(app_version); nothing compiled)"
  echo "DRY RUN PASSED"
  exit 0
fi
release_build
verify_app
publish_dmg
echo "DMG BUILT"
