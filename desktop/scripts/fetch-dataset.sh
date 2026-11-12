#!/usr/bin/env bash
# Downloads the PH laws dataset (laws.parquet, articles.parquet) at a pinned revision into
# data/source/ and writes data/dataset.lock (revision + SHA-256). Phase 3 §1.1.
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
