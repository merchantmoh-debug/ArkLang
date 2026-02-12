import unittest
import os
import json
import tempfile
import shutil
from meta.pkg.manifest import Manifest, PackageConfig, BuildConfig
from meta.pkg.builder import Builder

class TestPkg(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.manifest_path = os.path.join(self.test_dir, "ark.json")

    def tearDown(self):
        shutil.rmtree(self.test_dir)

    def test_manifest_load(self):
        data = {
            "package": {"name": "test", "version": "1.0.0"},
            "build": {"sources": ["main.ark"], "output": "out.ark"}
        }
        with open(self.manifest_path, "w") as f:
            json.dump(data, f)

        manifest = Manifest.load(self.manifest_path)
        self.assertEqual(manifest.package.name, "test")
        self.assertEqual(manifest.build.sources, ["main.ark"])

    def test_builder(self):
        # Create sources
        src_path = os.path.join(self.test_dir, "main.ark")
        with open(src_path, "w") as f:
            f.write("print('hello')")

        data = {
            "package": {"name": "test", "version": "1.0.0"},
            "build": {"sources": ["main.ark"], "output": "out.ark"}
        }
        with open(self.manifest_path, "w") as f:
            json.dump(data, f)

        manifest = Manifest.load(self.manifest_path)
        builder = Builder(manifest)
        builder.build()

        out_path = os.path.join(self.test_dir, "out.ark")
        self.assertTrue(os.path.exists(out_path))
        with open(out_path, "r") as f:
            content = f.read()
            self.assertIn("// Package: test", content)
            self.assertIn("print('hello')", content)

if __name__ == "__main__":
    unittest.main()
