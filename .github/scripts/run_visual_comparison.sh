#!/usr/bin/env bash
set -euo pipefail
REFERENCE_URL=${REFERENCE_URL:-http://127.0.0.1:8181/}
GPUI_URL=${GPUI_URL:-http://127.0.0.1:8182/}
OUT=${VISUAL_OUT:-visual-artifacts}
PORT=${CDP_PORT:-9223}
mkdir -p "$OUT"/{reference,actual,diff,summary}
node .github/scripts/capture_visual.mjs "$REFERENCE_URL" "$OUT/reference" reference "$PORT"
node .github/scripts/capture_visual.mjs "$GPUI_URL" "$OUT/actual" gpui "$PORT"
# Fail before invoking the comparator: an uncomposited WebGPU surface is a
# capture infrastructure error, never a meaningful visual regression.
python3 tests/visual/check_capture.py "$OUT"/reference/*.png "$OUT"/actual/*.png
status=0
for reference in "$OUT"/reference/*.png; do
  name=$(basename "$reference")
  python3 tests/visual/compare.py "$reference" "$OUT/actual/$name" \
    --diff "$OUT/diff/$name" --json "$OUT/summary/${name%.png}.json" \
    --channel-tolerance "${VISUAL_CHANNEL_TOLERANCE:-0}" \
    --max-difference "${VISUAL_MAX_DIFFERENCE:-32}" \
    --mean-difference "${VISUAL_MEAN_DIFFERENCE:-1.0}" \
    --changed-fraction "${VISUAL_CHANGED_FRACTION:-0.005}" || status=1
done
python3 - "$OUT" <<'PY'
import json, pathlib, sys
root = pathlib.Path(sys.argv[1])
items = [json.loads(p.read_text()) for p in sorted((root / "summary").glob("*.json"))]
(root / "summary.json").write_text(json.dumps({"passed": all(x["passed"] for x in items), "comparisons": items}, indent=2) + "\n")
PY
exit "$status"
