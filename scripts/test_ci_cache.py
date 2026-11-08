import copy
import json
import os
from pathlib import Path
import tempfile
import unittest

from ci_cache import restore, snapshot


class CompilerInputTests(unittest.TestCase):
    def test_only_matching_bytes_and_modes_reuse_timestamps(self):
        base = Path(__file__).resolve().parents[1] / ".tmp"
        base.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=base) as directory:
            root = Path(directory)
            paths = [root / name for name in ["same.rs", "changed.rs", "mode.rs"]]
            for path in paths:
                path.write_text("old")
                os.utime(path, ns=(1_000_000_000, 1_000_000_000))
            before = json.loads(json.dumps(snapshot(root, [p.name for p in paths])))
            for path in paths:
                os.utime(path, ns=(2_000_000_000, 2_000_000_000))
            paths[1].write_text("new")
            paths[2].chmod(0o700)
            (root / "link.rs").symlink_to(paths[0])
            current = snapshot(root, [p.name for p in paths] + ["link.rs"])
            # Changed bytes must invalidate even if a checkout preserved an
            # older timestamp than the cached compilation.
            os.utime(paths[1], ns=(1_000_000_000, 1_000_000_000))
            self.assertNotIn("link.rs", current["files"])
            self.assertEqual(restore(root, current, before), 1)
            self.assertEqual(paths[0].stat().st_mtime_ns, 1_000_000_000)
            self.assertGreater(paths[1].stat().st_mtime_ns, before["recorded_ns"])
            self.assertGreater(paths[2].stat().st_mtime_ns, before["recorded_ns"])

    def test_generated_directory_requires_identical_inventory_and_content(self):
        base = Path(__file__).resolve().parents[1] / ".tmp"
        base.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=base) as directory:
            root = Path(directory)
            dist = root / "web/dist"
            dist.mkdir(parents=True)
            (dist / "index.html").write_text("ui")
            os.utime(dist, ns=(1_000_000_000, 1_000_000_000))
            before = json.loads(json.dumps(snapshot(root, [])))
            os.utime(dist, ns=(2_000_000_000, 2_000_000_000))
            current = snapshot(root, [])
            restore(root, current, before)
            self.assertEqual(dist.stat().st_mtime_ns, 1_000_000_000)
            (dist / "link").symlink_to(dist / "index.html")
            restore(root, snapshot(root, []), before)
            self.assertGreater(dist.stat().st_mtime_ns, before["recorded_ns"])
            (dist / "link").unlink()
            (dist / "index.html").write_text("changed")
            os.utime(dist, ns=(2_000_000_000, 2_000_000_000))
            restore(root, snapshot(root, []), before)
            self.assertGreater(dist.stat().st_mtime_ns, before["recorded_ns"])
            (dist / "extra.js").write_text("new")
            restore(root, snapshot(root, []), before)
            self.assertGreater(dist.stat().st_mtime_ns, before["recorded_ns"])
            unsupported = copy.deepcopy(before)
            unsupported["format"] = 2
            with self.assertRaises(ValueError):
                restore(root, current, unsupported)
