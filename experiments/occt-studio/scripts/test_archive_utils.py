"""Regression tests for the actual release ZIP failure and archive boundaries."""
import hashlib
import os
from pathlib import Path
import tempfile
import unittest
import zipfile

from archive_utils import archive_tree, sha256_file, ZIP_EPOCH


class ArchiveTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.root = self.base / 'app'
        self.root.mkdir()
        self.file = self.root / 'LICENSE'
        self.file.write_bytes(b'license text\n')
        self.zip = self.base / 'app.zip'

    def test_pre_1980_timestamps_do_not_break_packaging(self):
        os.utime(self.file, (1, 1))
        archive_tree(self.root, self.zip)
        with zipfile.ZipFile(self.zip) as archive:
            self.assertEqual(archive.read('app/LICENSE'), b'license text\n')
            self.assertEqual(archive.getinfo('app/LICENSE').date_time, ZIP_EPOCH)
            self.assertIsNone(archive.testzip())

    def test_identical_content_is_deterministic_despite_mtime(self):
        (self.root / 'z.txt').write_bytes(b'Z' * 65536)
        archive_tree(self.root, self.zip)
        os.utime(self.file, (2_000_000_000, 2_000_000_000))
        second = self.base / 'second.zip'
        archive_tree(self.root, second)
        self.assertEqual(sha256_file(self.zip), sha256_file(second))
        with zipfile.ZipFile(self.zip) as archive:
            self.assertEqual(archive.namelist(), sorted(archive.namelist()))

    def test_existing_archive_is_not_overwritten(self):
        self.zip.write_bytes(b'keep me')
        with self.assertRaises(FileExistsError):
            archive_tree(self.root, self.zip)
        self.assertEqual(self.zip.read_bytes(), b'keep me')

    def test_destination_inside_tree_is_rejected(self):
        with self.assertRaises(ValueError):
            archive_tree(self.root, self.root / 'self.zip')

    def test_standalone_fonts_are_rejected_before_any_archive_is_created(self):
        (self.root / 'not-for-distribution.TTC').write_bytes(b'not a real font')
        with self.assertRaises(ValueError):
            archive_tree(self.root, self.zip)
        self.assertFalse(self.zip.exists())

    def test_symlinks_cannot_escape_staging_tree(self):
        try:
            (self.root / 'link').symlink_to(self.file)
        except OSError:
            self.skipTest('This runner does not allow symlinks')
        with self.assertRaises(ValueError):
            archive_tree(self.root, self.zip)
        self.assertFalse(self.zip.exists())

    def test_streaming_hash_matches_standard_hash(self):
        self.file.write_bytes(b'x' * (2 * 1024 * 1024 + 17))
        self.assertEqual(sha256_file(self.file), hashlib.sha256(self.file.read_bytes()).hexdigest())

    def test_empty_tree_has_valid_zip(self):
        self.file.unlink()
        archive_tree(self.root, self.zip)
        with zipfile.ZipFile(self.zip) as archive:
            self.assertEqual(archive.namelist(), [])


if __name__ == '__main__':
    unittest.main(verbosity=2)
