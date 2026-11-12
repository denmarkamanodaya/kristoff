#!/usr/bin/env bash
# Downloads the PH laws dataset (laws.parquet, articles.parquet) at a pinned revision into
# data/source/ and writes data/dataset.lock (revision + SHA-256). Phase 3 §1.1.
# Phase 7: also fetches OmniCorpus (mongramosjr/philippine-omnicorpus, gated) into
# data/source/omnicorpus/ and checks it against the committed data/omnicorpus.lock (never
# rewritten here). The Hugging Face token is read from ~/.kristoff-hf-token (or
# KRISTOFF_HF_TOKEN_FILE); it goes to curl through a header file, never argv or output.
# Usage: scripts/fetch-dataset.sh [data-dir]   (re-run with LOCK=verify to check sums only)
set -euo pipefail
DATA="${1:-$(cd "$(dirname "$0")/.." && pwd)/data}"
REPO="endomorphosis/ipfs_philippines_laws"
REV="66ffcff9f6fa1288ca5247d7b20141dc841beb45"
mkdir -p "$DATA/source"
for f in laws articles; do
  out="$DATA/source/$f.parquet"
  [ -s "$out" ] || curl -fsSL --retry 3 -o "$out.part" \
    "https://huggingface.co/datasets/$REPO/resolve/$REV/data/$f.parquet" && \
    { [ -f "$out.part" ] && mv "$out.part" "$out" || true; }
done
OMNI_REPO="mongramosjr/philippine-omnicorpus"
OMNI_REV="bb0626ce527b159a32a6baefabee0ea1e0e01d2f"
omni_dir="$DATA/source/omnicorpus"
omni="$omni_dir/philippine_laws.parquet"
mkdir -p "$omni_dir"
if [ ! -s "$omni" ]; then
  token_file="${KRISTOFF_HF_TOKEN_FILE:-$HOME/.kristoff-hf-token}"
  if [ ! -r "$token_file" ]; then
    echo "OmniCorpus is gated: accept its terms on huggingface.co, then save a read-only token to $token_file (chmod 600)" >&2
    exit 1
  fi
  curl -fsSL --retry 3 -o "$omni.part" \
    -H @<(printf 'Authorization: Bearer %s\n' "$(tr -d '[:space:]' < "$token_file")") \
    "https://huggingface.co/datasets/$OMNI_REPO/resolve/$OMNI_REV/data/philippine_laws.parquet"
  mv "$omni.part" "$omni"
fi
(cd "$omni_dir" && grep -E '^[0-9a-f]{64}  ' "$DATA/omnicorpus.lock" | shasum -a 256 -c -)
lock="$DATA/dataset.lock"
if [ "${LOCK:-write}" = verify ]; then
  (cd "$DATA/source" && grep -E '^[0-9a-f]{64}  ' "$lock" | shasum -a 256 -c -)
  exit 0
fi
{
  echo "# dataset: $REPO"
  echo "# revision: $REV"
  (cd "$DATA/source" && shasum -a 256 laws.parquet articles.parquet)
} > "$lock"
cat "$lock"
