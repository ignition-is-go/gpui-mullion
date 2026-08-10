# Paired screenshot regression

The `visual` CI job compares the browser demo in this checkout directly with the authoritative public
Leptos Mullion demo at commit `09a8b8cbe88521f5c975e42bc0d3104af5afa448`. It deliberately does not commit PNG goldens:
the pinned upstream revision is the reviewable source of truth, while the GPUI side is the exact commit that triggered CI.
Both revisions and a hash of the installed font manifest are stored in the artifact.

## Deterministic renderer

One Chrome Stable process renders both applications with a fresh origin state, device scale factor 1, SwiftShader, and the
same DejaVu/Liberation font installation. Both Trunk outputs are served with COOP `same-origin`, COEP `require-corp`, and
`Cache-Control: no-store`. Before every state the raw-CDP driver fixes the viewport, navigates, clears `localStorage` and
`sessionStorage`, reloads, waits for fonts and two animation frames, then settles for 500 ms.

The paired canonical files are:

- `initial-nested-3-panes` at 1280x720 and the compact 960x600 viewport;
- `vertical-rail-hovered-expanded`;
- `category-card-open`;
- `focus-unfocused-wash`;
- `command-palette-overlay`;
- `workspace-switch`.

All interactive states use CDP pointer or keyboard events, never an application mutation hook. The pinned demo does not
configure an auto-hide rail, and the GPUI demo does not expose a matching deterministic auto-hide configuration, so an
auto-hide compact/revealed pair is intentionally absent rather than fabricated. Add it once both public demos expose the
same reachable configuration.

## Pixel policy and artifacts

`tests/visual/compare.py` requires equal dimensions and reports maximum RGBA channel delta, mean channel delta, per-channel
maxima, changed pixel count, and changed fraction. It writes a red raw-error heatmap and JSON for every pair. Defaults are:

- channel tolerance: **0** (exact; no implicit anti-alias exemption);
- maximum channel difference: 32;
- mean channel difference: 1.0;
- changed fraction: 0.005.

All three limits must pass. If a future Chrome/font update demonstrates repeatable one-channel raster noise, a nonzero
`VISUAL_CHANNEL_TOLERANCE` must be justified in the reviewing change; it only affects the changed-pixel count and never hides
raw max/mean metrics. CI always uploads reference PNGs, actual PNGs, heatmaps, per-state JSON, combined JSON, environment
metadata, and process logs.

Run only the synthetic comparator check locally (it does not build or launch either app):

```sh
python3 tests/visual/test_compare.py
```

## Refreshing the reference pin

1. Review the upstream diff and choose a full public `ignition-is-go/mullion` commit SHA.
2. Change both `ref:` and `REFERENCE_COMMIT` in `.github/workflows/ci.yml`, and the SHA at the top of this document.
3. Run CI. Download `paired-visual-<sha>` and inspect every reference, actual, heatmap, and JSON result.
4. Record intentional presentation changes in the pull request. Do not replace the direct reference with opaque committed
   goldens, relax thresholds to accept a structural mismatch, or close the parity issue before a real CI run is green.
