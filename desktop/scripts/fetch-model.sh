#!/usr/bin/env bash
# Downloads intfloat/multilingual-e5-small at a pinned revision, converts the weights to f16
# safetensors (D-13: f16 on disk, f32 compute) and writes data/model.lock (revision + SHA-256).
# Conversion uses a throwaway uv environment; Python is not a project dependency.
# Usage: scripts/fetch-model.sh [data-dir]   (LOCK=verify checks sums only)
set -euo pipefail
DATA="${1:-$(cd "$(dirname "$0")/.." && pwd)/data}"
REPO="intfloat/multilingual-e5-small"
REV="614241f622f53c4eeff9890bdc4f31cfecc418b3"
DL="$DATA/model-download"; OUT="$DATA/model"
mkdir -p "$DL" "$OUT"
for f in config.json tokenizer.json model.safetensors; do
  [ -s "$DL/$f" ] || curl -fsSL --retry 3 -o "$DL/$f" "https://huggingface.co/$REPO/resolve/$REV/$f"
done
cp "$DL/config.json" "$DL/tokenizer.json" "$OUT/"
if [ ! -s "$OUT/model.safetensors" ]; then
  conv="$(mktemp -d)/to_f16.py"
  cat > "$conv" <<'PY'
import sys
from safetensors.numpy import load_file, save_file
t = load_file(sys.argv[1])
save_file({k: (v.astype("float16") if v.dtype.kind == "f" else v) for k, v in t.items()},
          sys.argv[2], metadata={"format": "pt"})
PY
  "${UV:-uv}" run --no-project --with safetensors==0.7.0 --with numpy==2.3.4 \
    python -I "$conv" "$DL/model.safetensors" "$OUT/model.safetensors"
fi
lock="$DATA/model.lock"
if [ "${LOCK:-write}" = verify ]; then
  (cd "$OUT" && grep -E '^[0-9a-f]{64}  ' "$lock" | shasum -a 256 -c -); exit 0
fi
{
  echo "# model: $REPO"
  echo "# revision: $REV"
  echo "# weights: f16 safetensors converted from the f32 original (sha below is of the f16 file)"
  echo "# original-f32: $(shasum -a 256 "$DL/model.safetensors" | cut -d' ' -f1)"
  (cd "$OUT" && shasum -a 256 config.json tokenizer.json model.safetensors)
} > "$lock"
cat "$lock"
