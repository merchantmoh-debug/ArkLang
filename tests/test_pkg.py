import unittest
from unittest.mock import patch, MagicMock
import os
import tempfile
import shutil
import tomllib
import tarfile
import io
from meta.pkg.cli import cmd_init, cmd_install, cmd_list, cmd_publish, cmd_search
from meta.pkg.manifest import Manifest
from meta.pkg.registry import Registry

class TestPkg(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.cwd = os.getcwd()
        os.chdir(self.test_dir)

    def tearDown(self):
        os.chdir(self.cwd)
        shutil.rmtree(self.test_dir)

    def test_init_creates_manifest(self):
        args = MagicMock()
        args.path = "."
        cmd_init(args)

        self.assertTrue(os.path.exists("ark.toml"))
        with open("ark.toml", "rb") as f:
            data = tomllib.load(f)

        self.assertEqual(data["package"]["name"], os.path.basename(self.test_dir))
        self.assertEqual(data["package"]["version"], "0.1.0")
        self.assertIn("dependencies", data)

    def test_init_already_exists(self):
        with open("ark.toml", "w") as f:
            f.write("[package]\nname=\"foo\"\n")

        args = MagicMock()
        args.path = "."

        # Capture stdout to verify error message
        with patch('sys.stdout', new=io.StringIO()) as fake_out:
            cmd_init(args)
            self.assertIn("already exists", fake_out.getvalue())

    def test_list_empty(self):
        with open("ark.toml", "w") as f:
            f.write("[package]\nname=\"foo\"\nversion=\"0.1.0\"\ndescription=\"\"\n\n[dependencies]\n")

        args = MagicMock()
        with patch('sys.stdout', new=io.StringIO()) as fake_out:
            cmd_list(args)
            output = fake_out.getvalue()
            self.assertIn("[dependencies]", output)
            self.assertEqual(output.strip(), "[dependencies]")

    @patch('urllib.request.urlopen')
    def test_install_mock(self, mock_urlopen):
        # Setup mock response
        mock_response = MagicMock()
        mock_response.read.return_value = b"test content"
        mock_response.__enter__.return_value = mock_response
        mock_urlopen.return_value = mock_response

        # Create ark.toml
        with open("ark.toml", "w") as f:
            f.write("[package]\nname=\"foo\"\nversion=\"0.1.0\"\ndescription=\"\"\n\n[dependencies]\n")

        args = MagicMock()
        args.name = "test-pkg"

        cmd_install(args)

        # Verify file downloaded
        target_path = os.path.join("lib", "test-pkg", "lib.ark")
        self.assertTrue(os.path.exists(target_path))
        with open(target_path, "rb") as f:
            self.assertEqual(f.read(), b"test content")

        # Verify manifest updated
        with open("ark.toml", "rb") as f:
            data = tomllib.load(f)
        self.assertIn("test-pkg", data["dependencies"])

    def test_publish_creates_tarball(self):
        # Create ark.toml
        with open("ark.toml", "w") as f:
            f.write("[package]\nname=\"foo\"\nversion=\"0.1.0\"\ndescription=\"\"\n\n[dependencies]\n")

        # Create some files
        with open("main.ark", "w") as f:
            f.write("print('hello')")
        os.makedirs(".git")
        with open(".git/config", "w") as f:
            f.write("git config")

        args = MagicMock()
        cmd_publish(args)

        filename = "foo-0.1.0.tar.gz"
        self.assertTrue(os.path.exists(filename))

        with tarfile.open(filename, "r:gz") as tar:
            names = tar.getnames()
            self.assertIn("./main.ark", names)
            self.assertIn("./ark.toml", names)
            # Should not contain .git
            for name in names:
                self.assertNotIn(".git", name)

if __name__ == "__main__":
    unittest.main()
