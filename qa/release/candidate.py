#!/usr/bin/env python3
"""Build and verify non-published Arclet CAD Windows/macOS release candidates.

This is a packaging integrity gate, NOT a CAD geometry or real-device certification.
No release, tag, notarization, signing identity, or automatic merge is created.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import plistlib
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
LICENCES = ("LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "THIRD-PARTY-LICENSES.txt")
PLATFORMS = ("windows-x64", "macos-arm64", "macos-intel")
BUNDLE = "Arclet CAD.app"
BUNDLE_ID = "io.github.07egta.arclet"
MAX_ARCHIVE_MEMBERS = 32
MAX_UNCOMPRESSED_BYTES = 800 * 1024 * 1024


class CandidateError(ValueError):
    """A candidate is missing, mislabeled, corrupt or unsafe."""


def sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def binary_kind(payload: bytes, platform: str) -> None:
    """Validate PE or Mach-O magic AND architecture, not just an executable suffix."""
    if len(payload) < 65_536:
        raise CandidateError("executable is implausibly small (<64 KiB)")
    if platform == "windows-x64":
        if payload[:2] != b"MZ" or len(payload) < 64:
            raise CandidateError("not a Windows PE executable")
        offset = struct.unpack_from("<I", payload, 0x3C)[0]
        if offset > len(payload) - 6 or payload[offset:offset + 4] != b"PE\x00\x00":
            raise CandidateError("missing or misplaced PE signature")
        if struct.unpack_from("<H", payload, offset + 4)[0] != 0x8664:
            raise CandidateError("PE must target x86_64")
    else:
        if payload[:4] != b"\xcf\xfa\xed\xfe":
            raise CandidateError("expected little-endian 64-bit Mach-O binary")
        cpu = struct.unpack_from("<I", payload, 4)[0]
        expected = 0x0100000C if platform == "macos-arm64" else 0x01000007
        if cpu != expected:
            raise CandidateError(f"Mach-O architecture mismatch: {cpu:#x}, expected {expected:#x}")


def expected_paths(platform: str) -> tuple[str, str]:
    if platform not in PLATFORMS:
        raise CandidateError(f"unknown target platform: {platform}")
    if platform == "windows-x64":
        return "ArcletCAD.exe", "manifest.json"
    return f"{BUNDLE}/Contents/MacOS/arclet", f"{BUNDLE}/Contents/Resources/manifest.json"


def plist(version: str) -> bytes:
    return plistlib.dumps({
        "CFBundleExecutable": "arclet",
        "CFBundleIdentifier": BUNDLE_ID,
        "CFBundleName": "Arclet CAD",
        "CFBundleDisplayName": "Arclet CAD",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": version,
        "CFBundleVersion": "1",
        "LSMinimumSystemVersion": "11.0",
        "NSHighResolutionCapable": True,
    })


def package_candidate(
    binary: Path, dest: Path, platform: str, source_sha: str,
    root: Path = ROOT, adhoc_sign: bool = False,
) -> Path:
    executable, manifest_path = expected_paths(platform)
    if not re.fullmatch(r"[a-f0-9]{40}", source_sha):
        raise CandidateError("source SHA must be a full lowercase Git commit hash")
    if adhoc_sign and platform == "windows-x64":
        raise CandidateError("macOS ad-hoc signing cannot be used for Windows")
    for file_name in LICENCES:
        if not (root / file_name).is_file():
            raise CandidateError(f"licence file missing: {file_name}")
    if not binary.is_file():
        raise CandidateError(f"compiled binary is missing: {binary}")
    data = binary.read_bytes()
    binary_kind(data, platform)
    version = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    manifest = {
        "schema": "arclet-candidate/1",
        "platform": platform,
        "version": version,
        "source_commit": source_sha,
        "binary_sha256": sha256(data),
        "signature": "ad-hoc" if adhoc_sign else "none",
        "verified_geometry": False,
        "notarized": False,
    }
    dest.mkdir(parents=True, exist_ok=True)
    output = dest / f"ArcletCAD-{platform}-{source_sha[:8]}.zip"
    with tempfile.TemporaryDirectory(prefix="arclet-candidate-") as tmp:
        stage = Path(tmp)
        exe = stage / executable
        exe.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(binary, exe)
        if platform != "windows-x64":
            exe.chmod(0o755)
            info = stage / BUNDLE / "Contents/Info.plist"
            info.write_bytes(plist(version))
        resources = stage if platform == "windows-x64" else stage / BUNDLE / "Contents/Resources"
        resources.mkdir(parents=True, exist_ok=True)
        for file_name in LICENCES:
            shutil.copyfile(root / file_name, resources / file_name)
        (stage / manifest_path).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        readme = (
            f"Arclet CAD {version} development candidate for {platform}\n"
            f"Source commit: {source_sha}\n"
            "CAD results, saves and GPU support still require independent acceptance tests.\n"
            "Windows: unsigned executable. macOS: ad-hoc signed if specified, NOT notarized.\n"
            "Do not publish as a verified professional release solely from this archive.\n"
        )
        (resources / "README.txt").write_text(readme, encoding="utf-8")
        if adhoc_sign:
            if sys.platform != "darwin":
                raise CandidateError("ad-hoc codesign must run on macOS")
            # Sign the Mach-O itself, then record the post-signing binary digest.
            # The enclosing .app bundle is NOT Developer-ID signed or notarized.
            subprocess.run(["codesign", "--force", "--sign", "-", "--timestamp=none", str(exe)], check=True)
            subprocess.run(["codesign", "--verify", "--strict", str(exe)], check=True)
            manifest["binary_sha256"] = sha256(exe.read_bytes())
            (stage / manifest_path).write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED, compresslevel=6, allowZip64=True) as archive:
            for path in sorted(stage.rglob("*")):
                if not path.is_file():
                    continue
                relative = path.relative_to(stage).as_posix()
                item = zipfile.ZipInfo(relative, date_time=(1980, 1, 1, 0, 0, 0))
                item.compress_type = zipfile.ZIP_DEFLATED
                item.external_attr = ((0o755 if relative == executable else 0o644) << 16)
                archive.writestr(item, path.read_bytes())
    checksum = output.with_name(output.name + ".sha256")
    checksum.write_text(f"{sha256(output.read_bytes())}  {output.name}\n", encoding="ascii")
    verify_candidate(output, platform, source_sha)
    return output


def verify_candidate(archive_path: Path, platform: str, source_sha: str | None = None) -> dict:
    executable, manifest_path = expected_paths(platform)
    checksum = archive_path.with_name(archive_path.name + ".sha256")
    if not archive_path.is_file() or not checksum.is_file():
        raise CandidateError("archive or separate SHA-256 checksum file is missing")
    text = checksum.read_text(encoding="ascii").strip()
    match = re.fullmatch(r"([a-f0-9]{64})  ([A-Za-z0-9_.-]+\.zip)", text)
    if match is None or match.group(2) != archive_path.name or sha256(archive_path.read_bytes()) != match.group(1):
        raise CandidateError("archive SHA-256 check failed")
    with zipfile.ZipFile(archive_path) as archive:
        files = [item for item in archive.infolist() if not item.is_dir()]
        names = [item.filename for item in files]
        if not files or len(files) > MAX_ARCHIVE_MEMBERS or len(set(names)) != len(names):
            raise CandidateError("empty, oversized or duplicate-name archive")
        if sum(item.file_size for item in files) > MAX_UNCOMPRESSED_BYTES:
            raise CandidateError("archive uncompressed size limit exceeded")
        for item in files:
            name = item.filename
            parts = PurePosixPath(name).parts
            if not name or "\\" in name or name.startswith("/") or ".." in parts or name.startswith("./"):
                raise CandidateError(f"unsafe archive entry: {name}")
            if (item.external_attr >> 16) & 0o170000 == 0o120000:
                raise CandidateError(f"symbolic link not permitted in package: {name}")
        prefix = "" if platform == "windows-x64" else f"{BUNDLE}/Contents/Resources/"
        required = {executable, manifest_path, *(prefix + file_name for file_name in LICENCES), prefix + "README.txt"}
        if platform != "windows-x64":
            required.add(f"{BUNDLE}/Contents/Info.plist")
        if not required.issubset(names):
            raise CandidateError(f"missing mandatory files: {sorted(required.difference(names))}")
        allowed_signature_prefix = f"{BUNDLE}/Contents/_CodeSignature/"
        extras = set(names) - required
        if extras and (platform == "windows-x64" or any(not path.startswith(allowed_signature_prefix) for path in extras)):
            raise CandidateError(f"unexpected archive contents: {sorted(extras)}")
        manifest = json.loads(archive.read(manifest_path))
        if manifest.get("schema") != "arclet-candidate/1" or manifest.get("platform") != platform:
            raise CandidateError("incorrect candidate manifest/schema/platform")
        if source_sha is not None and manifest.get("source_commit") != source_sha:
            raise CandidateError("candidate was built from the wrong source commit")
        if manifest.get("notarized") is not False or manifest.get("verified_geometry") is not False:
            raise CandidateError("candidate makes unsupported verification claims")
        if manifest.get("signature") not in ("none", "ad-hoc"):
            raise CandidateError("unknown signing status")
        if manifest["signature"] == "ad-hoc" and platform == "windows-x64":
            raise CandidateError("Windows binary cannot carry an ad-hoc macOS signature")
        binary = archive.read(executable)
        binary_kind(binary, platform)
        if sha256(binary) != manifest.get("binary_sha256"):
            raise CandidateError("binary hash differs from signed-off manifest")
        if platform != "windows-x64":
            info = plistlib.loads(archive.read(f"{BUNDLE}/Contents/Info.plist"))
            if (info.get("CFBundleExecutable") != "arclet"
                or info.get("CFBundleIdentifier") != BUNDLE_ID
                or info.get("CFBundleShortVersionString") != manifest.get("version")):
                raise CandidateError("invalid macOS bundle identity/version")
        return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("package")
    build.add_argument("--binary", type=Path, required=True)
    build.add_argument("--out", type=Path, required=True)
    build.add_argument("--platform", choices=PLATFORMS, required=True)
    build.add_argument("--source-sha", required=True)
    build.add_argument("--adhoc-sign", action="store_true")
    verify = commands.add_parser("verify")
    verify.add_argument("--archive", type=Path, required=True)
    verify.add_argument("--platform", choices=PLATFORMS, required=True)
    verify.add_argument("--source-sha", required=True)
    args = parser.parse_args()
    try:
        if args.command == "package":
            archive = package_candidate(args.binary, args.out, args.platform, args.source_sha, adhoc_sign=args.adhoc_sign)
            print(f"candidate packaged and verified: {archive}")
        else:
            manifest = verify_candidate(args.archive, args.platform, args.source_sha)
            print(f"candidate integrity verified: {manifest['platform']} @ {manifest['source_commit']}")
    except (CandidateError, OSError, ValueError, KeyError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        print(f"release candidate FAILED: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
