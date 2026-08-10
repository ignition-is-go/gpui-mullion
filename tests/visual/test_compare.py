#!/usr/bin/env python3
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from PIL import Image

compare = Path(__file__).with_name("compare.py")
with tempfile.TemporaryDirectory() as temporary:
    root = Path(temporary)
    equal = Image.new("RGBA", (3, 2), (10, 20, 30, 255))
    equal.save(root / "reference.png")
    equal.save(root / "equal.png")
    changed = equal.copy()
    changed.putpixel((1, 1), (250, 20, 30, 255))
    changed.save(root / "changed.png")
    common = [sys.executable, str(compare), str(root / "reference.png")]
    ok = subprocess.run(common + [str(root / "equal.png"), "--diff", str(root / "equal-diff.png"),
                        "--json", str(root / "equal.json")], check=False)
    bad = subprocess.run(common + [str(root / "changed.png"), "--diff", str(root / "bad-diff.png"),
                         "--json", str(root / "bad.json"), "--max-difference", "0",
                         "--mean-difference", "0", "--changed-fraction", "0"], check=False)
    summary = json.loads((root / "bad.json").read_text())
    assert ok.returncode == 0
    assert bad.returncode != 0
    assert summary["changed_pixels"] == 1 and summary["max_channel_difference"] == 240
print("comparator self-test passed (equal accepted, deliberate change rejected)")
