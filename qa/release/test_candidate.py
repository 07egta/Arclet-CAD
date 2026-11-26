"""Original release-candidate fixtures: valid PE/Mach-O headers, and hostile archives."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile
import json
import struct
import hashlib

SPEC = importlib.util.spec_from_file_location("candidate", Path(__file__).with_name("candidate.py"))
candidate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(candidate)
SHA = "0123456789abcdef" * 2 + "01234567"


def binary(platform):
    data = bytearray(65536)
    if platform == "windows-x64":
        data[0:2] = b"MZ"
        struct.pack_into("<I", data, 0x3c, 128)
        data[128:132] = b"PE\x00\x00"
        struct.pack_into("<H", data, 132, 0x8664)
    else:
        data[0:4] = b"\xcf\xfa\xed\xfe"
        struct.pack_into("<I", data, 4, 0x0100000C if platform == "macos-arm64" else 0x01000007)
    return bytes(data)


class CandidateTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / "apps/arclet").mkdir(parents=True)
        (self.root / "Cargo.toml").write_text('[workspace.package]\nversion = "0.1.0"\n')
        for file in candidate.LICENCES:
            (self.root / file).write_text("Original fixture\n")
        self.exe = self.root / "arclet.bin"

    def make(self, platform):
        self.exe.write_bytes(binary(platform))
        return candidate.package_candidate(self.exe, self.root / "out", platform, SHA, root=self.root)

    def test_all_platform_formats_and_checksums(self):
        for platform in candidate.PLATFORMS:
            with self.subTest(platform=platform):
                path = self.make(platform)
                manifest = candidate.verify_candidate(path, platform, SHA)
                self.assertEqual(manifest["source_commit"], SHA)
                self.assertFalse(manifest["notarized"])
                self.assertFalse(manifest["verified_geometry"])

    def test_corrupted_checksum_fails(self):
        path = self.make("windows-x64")
        path.with_name(path.name + ".sha256").write_text("0" * 64 + "  " + path.name + "\n")
        with self.assertRaises(candidate.CandidateError):
            candidate.verify_candidate(path, "windows-x64", SHA)

    def test_wrong_architecture_or_wrong_commit_fails(self):
        path = self.make("macos-arm64")
        with self.assertRaises(candidate.CandidateError):
            candidate.verify_candidate(path, "macos-intel", SHA)
        with self.assertRaises(candidate.CandidateError):
            candidate.verify_candidate(path, "macos-arm64", "f" * 40)
        with self.assertRaises(candidate.CandidateError):
            candidate.binary_kind(binary("macos-intel"), "macos-arm64")

    def test_no_unexpected_files(self):
        path = self.make("windows-x64")
        checksum = path.with_name(path.name + ".sha256")
        with zipfile.ZipFile(path, "a") as z:
            z.writestr("../outside.txt", "unsafe")
        checksum.write_text(hashlib.sha256(path.read_bytes()).hexdigest() + "  " + path.name + "\n")
        with self.assertRaises(candidate.CandidateError):
            candidate.verify_candidate(path, "windows-x64", SHA)


if __name__ == "__main__":
    unittest.main()
