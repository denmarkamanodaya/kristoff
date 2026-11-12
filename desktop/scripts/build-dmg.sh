#!/usr/bin/env bash
# Builds the release DMG for Kristoff Desktop (docs/desktop/phases/phase-5-eval-package.md §2):
# checks the baked Claude API key (never printed; no TOTP secret since decision D-17), verifies
# the law index, the decisions index (Phase 7) and the e5 model against their lock files, checks THIRD_PARTY_NOTICES, builds the desktop web bundle, builds the arm64
# app and DMG with the bundled resources, verifies the ad-hoc signature, and writes
# desktop/dist/Kristoff-<version>-arm64.dmg plus its .sha256.
#
# Usage: KRISTOFF_ANTHROPIC_KEY=... bash desktop/scripts/build-dmg.sh [--dry-run]
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
DATA="data"

step() { echo "==> $*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

check_secrets() {
  step "Claude API key (the value is never printed)"
  local key="${KRISTOFF_ANTHROPIC_KEY:-}"
  [ -n "$key" ] || fail "KRISTOFF_ANTHROPIC_KEY is not set"
  case "$key" in
    sk-ant-test*) fail "KRISTOFF_ANTHROPIC_KEY is the fake dev key" ;;
    sk-ant-*) ;;
    *) fail "KRISTOFF_ANTHROPIC_KEY does not look like a Claude API key (sk-ant-...)" ;;
  esac
  echo "ok: set, and not the dev key"
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

meta() { db_meta phlaws.db "$1"; }
db_meta() { sqlite3 "$DATA/$1" "SELECT value FROM meta WHERE key = '$2'"; }

check_omnicorpus_sources() {
  if [ -d "$DATA/source/omnicorpus" ]; then
    step "OmniCorpus parquet files against omnicorpus.lock"
    (cd "$DATA/source/omnicorpus" && grep -E '^[0-9a-f]{64}  ' ../../omnicorpus.lock | shasum -a 256 -c -) \
      || fail "OmniCorpus files do not match omnicorpus.lock"
  fi
}

# Both databases carry the OmniCorpus revision they were built from (meta `omni_revision`, Phase 7).
check_phcases() {
  step "phcases.db against the lock revisions"
  local db=phcases.db omni cases passages
  omni="$(lock_value omnicorpus.lock revision)"
  [ -s "$DATA/$db" ] || fail "$DATA/$db is missing (run kristoff-ingest --cases-out)"
  [ "$(meta omni_revision)" = "$omni" ] || fail "phlaws.db was built from another OmniCorpus revision"
  [ "$(db_meta $db omni_revision)" = "$omni" ] || fail "$db was built from another OmniCorpus revision"
  [ "$(db_meta $db model_revision)" = "$(lock_value model.lock revision)" ] \
    || fail "$db was built with another model revision"
  for stage in cases_stage fts_stage; do
    [ "$(db_meta $db $stage)" = "done" ] || fail "$db ingest did not finish ($stage)"
  done
  cases="$(db_meta $db case_count)"; passages="$(db_meta $db passage_count)"
  [ "${cases:-0}" -gt 0 ] || fail "$db has no case_count"
  [ "${passages:-0}" -gt 0 ] || fail "$db has no passage_count"
  [ "$(sqlite3 "$DATA/$db" 'PRAGMA quick_check')" = "ok" ] || fail "$db fails quick_check"
  echo "ok: OmniCorpus ${omni:0:8}, $cases decisions, $passages passages, built $(db_meta $db build_date)"
}

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
  # Capture first: `grep -q` exits early, codesign then gets SIGPIPE and pipefail reports a failure.
  local sig
  sig="$(codesign -dv "$app" 2>&1)"
  [[ "$sig" == *"Signature=adhoc"* ]] || fail "the app is not ad-hoc signed"
  for res in phlaws.db phcases.db models/e5-small/model.safetensors THIRD_PARTY_NOTICES; do
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
check_omnicorpus_sources
check_phcases
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
