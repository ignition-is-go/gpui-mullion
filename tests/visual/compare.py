#!/usr/bin/env python3
"""Strict paired PNG comparator used by the visual regression job."""
import argparse
import json
import sys
from pathlib import Path
from PIL import Image, ImageChops


def compare(reference, actual, diff, channel_tolerance=0):
    ref = Image.open(reference).convert("RGBA")
    got = Image.open(actual).convert("RGBA")
    uniform = [name for name, image in (("reference", ref), ("actual", got))
               if all(low == high for low, high in image.convert("RGB").getextrema())]
    if uniform:
        Path(diff).parent.mkdir(parents=True, exist_ok=True)
        Image.new("RGBA", ref.size, (255, 255, 0, 255)).save(diff)
        return {"reference": str(reference), "actual": str(actual), "diff": str(diff),
                "error": f"uniform {' and '.join(uniform)} capture", "passed": False}
    if ref.size != got.size:
        # A magenta canvas makes a dimension failure visible in artifacts even
        # though corresponding-pixel statistics would be meaningless.
        size = (max(ref.width, got.width), max(ref.height, got.height))
        heat = Image.new("RGBA", size, (255, 0, 255, 255))
        Path(diff).parent.mkdir(parents=True, exist_ok=True)
        heat.save(diff)
        return {"reference": str(reference), "actual": str(actual), "diff": str(diff),
                "reference_size": list(ref.size), "actual_size": list(got.size),
                "error": "dimension mismatch", "passed": False}
    rp, ap = list(ref.getdata()), list(got.getdata())
    pixel_count = len(rp)
    channel_diffs = [abs(a - b) for left, right in zip(rp, ap) for a, b in zip(left, right)]
    per_channel_max = [max(abs(left[c] - right[c]) for left, right in zip(rp, ap)) for c in range(4)]
    changed = sum(any(abs(a - b) > channel_tolerance for a, b in zip(left, right))
                  for left, right in zip(rp, ap))
    raw_max = max(channel_diffs, default=0)
    raw_mean = sum(channel_diffs) / max(1, len(channel_diffs))
    # Heatmap is deliberately based on raw RGB error, not tolerance-adjusted error.
    heat = Image.new("RGBA", ref.size)
    heat.putdata([(max(abs(a - b) for a, b in zip(left[:3], right[:3])), 0, 0, 255)
                  for left, right in zip(rp, ap)])
    Path(diff).parent.mkdir(parents=True, exist_ok=True)
    heat.save(diff)
    return {"reference": str(reference), "actual": str(actual), "diff": str(diff),
            "dimensions": list(ref.size), "pixels": pixel_count,
            "channel_tolerance": channel_tolerance, "per_channel_max_rgba": per_channel_max,
            "max_channel_difference": raw_max, "mean_channel_difference": raw_mean,
            "changed_pixels": changed, "changed_fraction": changed / max(1, pixel_count)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("reference", type=Path)
    parser.add_argument("actual", type=Path)
    parser.add_argument("--diff", type=Path, required=True)
    parser.add_argument("--json", type=Path, required=True)
    parser.add_argument("--channel-tolerance", type=int, default=0,
                        help="ignore a channel only when its absolute delta is <= N (default: exact)")
    parser.add_argument("--max-difference", type=int, default=32)
    parser.add_argument("--mean-difference", type=float, default=1.0)
    parser.add_argument("--changed-fraction", type=float, default=0.005)
    args = parser.parse_args()
    if not 0 <= args.channel_tolerance <= 255:
        parser.error("--channel-tolerance must be in [0,255]")
    result = compare(args.reference, args.actual, args.diff, args.channel_tolerance)
    thresholds = {"max_difference": args.max_difference, "mean_difference": args.mean_difference,
                  "changed_fraction": args.changed_fraction}
    result["thresholds"] = thresholds
    result["passed"] = not result.get("error") and (
        result["max_channel_difference"] <= args.max_difference
        and result["mean_channel_difference"] <= args.mean_difference
        and result["changed_fraction"] <= args.changed_fraction)
    args.json.parent.mkdir(parents=True, exist_ok=True)
    args.json.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps(result, sort_keys=True))
    return 0 if result["passed"] else 1

if __name__ == "__main__":
    sys.exit(main())
