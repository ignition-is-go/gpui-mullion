#!/usr/bin/env python3
"""Strict paired PNG comparator used by the visual regression job."""
import argparse
import json
import sys
from pathlib import Path
from PIL import Image


def save_blink(reference, actual, path):
    """Write a reviewable alternating reference/candidate animation."""
    size = (max(reference.width, actual.width), max(reference.height, actual.height))
    frames = []
    for image in (reference, actual):
        frame = Image.new("RGBA", size, (255, 0, 255, 255))
        frame.paste(image, (0, 0))
        frames.append(frame.convert("P", palette=Image.Palette.ADAPTIVE))
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    frames[0].save(path, save_all=True, append_images=frames[1:], duration=500, loop=0)


def compare(reference, actual, diff, channel_tolerance=0, blink=None):
    ref = Image.open(reference).convert("RGBA")
    got = Image.open(actual).convert("RGBA")
    blink = Path(blink) if blink else Path(diff).with_suffix(".blink.gif")
    save_blink(ref, got, blink)
    heat_path = Path(diff).with_suffix(".heat.png")
    common = {"reference": str(reference), "actual": str(actual), "diff": str(diff),
              "heat": str(heat_path), "blink": str(blink)}
    uniform = [name for name, image in (("reference", ref), ("actual", got))
               if all(low == high for low, high in image.convert("RGB").getextrema())]
    if uniform:
        Path(diff).parent.mkdir(parents=True, exist_ok=True)
        marker = Image.new("RGBA", ref.size, (255, 255, 0, 255))
        marker.save(diff)
        marker.save(heat_path)
        return {**common, "difference_bounds": None,
                "error": f"uniform {' and '.join(uniform)} capture", "passed": False}
    if ref.size != got.size:
        # A magenta canvas makes a dimension failure visible in artifacts even
        # though corresponding-pixel statistics would be meaningless.
        size = (max(ref.width, got.width), max(ref.height, got.height))
        heat = Image.new("RGBA", size, (255, 0, 255, 255))
        Path(diff).parent.mkdir(parents=True, exist_ok=True)
        heat.save(diff)
        heat.save(heat_path)
        return {**common, "difference_bounds": [0, 0, *size],
                "reference_size": list(ref.size), "actual_size": list(got.size),
                "error": "dimension mismatch", "passed": False}
    rp, ap = list(ref.getdata()), list(got.getdata())
    pixel_count = len(rp)
    raw_diff_pixels = [tuple(abs(a - b) for a, b in zip(left, right))
                       for left, right in zip(rp, ap)]
    channel_diffs = [value for pixel in raw_diff_pixels for value in pixel]
    per_channel_max = [max(abs(left[c] - right[c]) for left, right in zip(rp, ap)) for c in range(4)]
    changed = sum(any(value > channel_tolerance for value in pixel)
                  for pixel in raw_diff_pixels)
    raw_max = max(channel_diffs, default=0)
    raw_mean = sum(channel_diffs) / max(1, len(channel_diffs))
    raw_mask = Image.new("L", ref.size)
    raw_mask.putdata([max(pixel) for pixel in raw_diff_pixels])
    difference_bounds = list(raw_mask.getbbox()) if raw_mask.getbbox() else None
    absolute = Image.new("RGBA", ref.size)
    absolute.putdata(raw_diff_pixels)
    Path(diff).parent.mkdir(parents=True, exist_ok=True)
    absolute.save(diff)
    heat = Image.new("RGBA", ref.size)
    heat.putdata([(max(pixel), 0, 0, 255) for pixel in raw_diff_pixels])
    heat.save(heat_path)
    return {**common, "difference_bounds": difference_bounds,
            "dimensions": list(ref.size), "pixels": pixel_count,
            "channel_tolerance": channel_tolerance, "per_channel_max_rgba": per_channel_max,
            "max_channel_difference": raw_max, "mean_channel_difference": raw_mean,
            "changed_pixels": changed, "changed_fraction": changed / max(1, pixel_count)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("reference", type=Path)
    parser.add_argument("actual", type=Path)
    parser.add_argument("--diff", type=Path, required=True,
                        help="absolute RGBA difference PNG")
    parser.add_argument("--blink", type=Path,
                        help="alternating reference/candidate GIF (default: beside --diff)")
    parser.add_argument("--json", type=Path, required=True)
    parser.add_argument("--channel-tolerance", type=int, default=0,
                        help="ignore a channel only when its absolute delta is <= N (default: exact)")
    parser.add_argument("--max-difference", type=int, default=32)
    parser.add_argument("--mean-difference", type=float, default=1.0)
    parser.add_argument("--changed-fraction", type=float, default=0.005)
    args = parser.parse_args()
    if not 0 <= args.channel_tolerance <= 255:
        parser.error("--channel-tolerance must be in [0,255]")
    result = compare(args.reference, args.actual, args.diff, args.channel_tolerance, args.blink)
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
