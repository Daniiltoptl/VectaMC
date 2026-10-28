"""Regression tests for vanilla-reference download and extraction integrity."""

import hashlib
from pathlib import Path
import tempfile
import unittest
import zipfile

from prepare_vanilla import download, extract_sources


class ReferenceIntegrityTests(unittest.TestCase):
    def test_bad_download_does_not_replace_existing_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source.jar"
            source.write_bytes(b"corrupted download")
            destination = root / "cached.jar"
            destination.write_bytes(b"previous contents")
            expected = hashlib.sha256(b"expected contents").hexdigest()
            with self.assertRaisesRegex(ValueError, "Checksum or size mismatch"):
                download(source.as_uri(), destination, expected, "sha256")
            self.assertEqual(destination.read_bytes(), b"previous contents")
            self.assertFalse(destination.with_suffix(".jar.part").exists())

    def test_corrupt_cache_is_replaced_and_valid_cache_is_reusable_offline(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source.jar"
            source.write_bytes(b"official contents")
            destination = root / "cached.jar"
            destination.write_bytes(b"corrupted cache")
            expected = hashlib.sha1(source.read_bytes()).hexdigest()
            download(source.as_uri(), destination, expected, "sha1", source.stat().st_size)
            source.unlink()
            download(source.as_uri(), destination, expected, "sha1", destination.stat().st_size)
            self.assertEqual(destination.read_bytes(), b"official contents")

    def test_archive_paths_cannot_escape_reference_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "sources.jar"
            with zipfile.ZipFile(archive, "w") as output:
                output.writestr("../outside.java", "malicious")
            with self.assertRaisesRegex(ValueError, "Unsafe archive entry"):
                extract_sources(archive, root / "reference")
            self.assertFalse((root / "outside.java").exists())

    def test_extraction_keeps_sources_and_skips_binary_assets(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "sources.jar"
            with zipfile.ZipFile(archive, "w") as output:
                output.writestr("net/minecraft/Test.java", "class Test {}")
                output.writestr("assets/example.bin", b"binary")
            destination = root / "reference"
            self.assertEqual(extract_sources(archive, destination), 1)
            self.assertEqual((destination / "net/minecraft/Test.java").read_text(), "class Test {}")
            self.assertFalse((destination / "assets").exists())


if __name__ == "__main__":
    unittest.main()
