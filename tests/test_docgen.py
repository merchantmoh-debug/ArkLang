import unittest
import os
import sys
import shutil
import tempfile
from unittest.mock import patch, MagicMock

# Add repo root to sys.path
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if REPO_ROOT not in sys.path:
    sys.path.append(REPO_ROOT)

from meta.docgen import DocGenerator

class TestDocGen(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.docgen = DocGenerator(output_dir=self.test_dir)

    def tearDown(self):
        shutil.rmtree(self.test_dir)

    def test_parse_intrinsics(self):
        # Mock INTRINSICS
        mock_intrinsics = {
            "sys.test_func": lambda x: x,
            "sys.fs.read": lambda x: x,
            "math.add": lambda x: x,
            "sys.exec": lambda x: x
        }
        with patch("meta.docgen.INTRINSICS", mock_intrinsics):
            self.docgen.parse_intrinsics()

            # sys.test_func -> category 'sys' (because only 2 parts and not explicitly handled except default to sys if startswith sys?)
            # Wait, sys.test_func -> parts=['sys', 'test_func']. len=2.
            # else: category="sys"
            self.assertIn("sys", self.docgen.intrinsics_data)
            self.assertEqual(next(i for i in self.docgen.intrinsics_data["sys"] if i["name"] == "sys.test_func")["name"], "sys.test_func")

            # sys.fs.read -> category 'fs'
            self.assertIn("fs", self.docgen.intrinsics_data)
            self.assertEqual(self.docgen.intrinsics_data["fs"][0]["name"], "sys.fs.read")

            # math.add -> category 'math'
            self.assertIn("math", self.docgen.intrinsics_data)
            self.assertEqual(self.docgen.intrinsics_data["math"][0]["name"], "math.add")

            # sys.exec -> category 'sys'
            # self.assertIn("sys", self.docgen.intrinsics_data) # Already checked
            self.assertEqual(next(i for i in self.docgen.intrinsics_data["sys"] if i["name"] == "sys.exec")["name"], "sys.exec")

    def test_parse_stdlib(self):
        # Create a dummy .ark file
        os.makedirs(os.path.join(self.test_dir, "std"), exist_ok=True)
        filepath = os.path.join(self.test_dir, "std", "dummy.ark")
        with open(filepath, "w") as f:
            f.write("""
// This is a test function
// It does nothing
func dummy_func(a, b) {
    return a + b
}

func no_doc_func() {
    return 0
}
            """)

        self.docgen.parse_stdlib(stdlib_path=os.path.join(self.test_dir, "std"))

        self.assertIn("dummy", self.docgen.stdlib_data)
        funcs = self.docgen.stdlib_data["dummy"]
        self.assertEqual(len(funcs), 2)

        func1 = next(f for f in funcs if f["name"] == "dummy_func")
        self.assertEqual(func1["args"], "a, b")
        self.assertIn("This is a test function", func1["doc"])

        func2 = next(f for f in funcs if f["name"] == "no_doc_func")
        self.assertEqual(func2["doc"], "No description available.")

    def test_generate_files(self):
        # Populate with dummy data
        self.docgen.intrinsics_data = {"core": [{"name": "print", "params": "(...)", "doc": "Prints stuff"}]}
        self.docgen.stdlib_data = {"math": [{"name": "add", "args": "a, b", "doc": "Adds numbers"}]}

        self.docgen.generate_api_docs()
        self.docgen.generate_stdlib_docs()
        self.docgen.generate_quick_start()

        self.assertTrue(os.path.exists(os.path.join(self.test_dir, "API_REFERENCE.md")))
        self.assertTrue(os.path.exists(os.path.join(self.test_dir, "STDLIB_REFERENCE.md")))
        self.assertTrue(os.path.exists(os.path.join(self.test_dir, "QUICK_START.md")))

        # Check content
        with open(os.path.join(self.test_dir, "API_REFERENCE.md"), "r") as f:
            content = f.read()
            self.assertIn("# Ark API Reference", content)
            self.assertIn("print", content)

    def test_html_generation(self):
        self.docgen.format = "html"
        self.docgen.generate_quick_start()
        self.docgen.generate_index_html()

        filepath = os.path.join(self.test_dir, "QUICK_START.html")
        self.assertTrue(os.path.exists(filepath))
        with open(filepath, "r") as f:
            content = f.read()
            self.assertIn("<html>", content)
            self.assertIn("highlight.min.js", content)
            # Check for header IDs
            self.assertIn('id="installation"', content)

        index_path = os.path.join(self.test_dir, "index.html")
        self.assertTrue(os.path.exists(index_path))
        with open(index_path, "r") as f:
            content = f.read()
            # Check for links
            self.assertIn('<a href="QUICK_START.html">Quick Start</a>', content)

if __name__ == "__main__":
    unittest.main()
