"""Distribution source-code provenance and ZIP-integrity regressions."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys
import zipfile

import pytest

EXPERIMENT = Path(__file__).resolve().parents[1]
SCRIPTS = EXPERIMENT / "scripts"
sys.path.insert(0, str(SCRIPTS))
import package_windows as pack

# Never execute or load the dummy native DLL. This verifies actual upstream
# UI source bytes independently of the Windows operating system.
UPSTREAM = Path.cwd()
UI_HASH = json.loads((EXPERIMENT / "upstream.json").read_text(encoding="utf-8"))["protected_ui_sha256"]


def make_archive(path, *, change_ui=False, change_dll=False, extra=None, omit=None):
    dll = b"verified dll content"
    source = b"different native image" if change_dll else dll
    prefix = f"{pack.KIT}/"
    spec = {
        "upstream_commit": "pinned",
        "protected_ui_files": 64,
        "protected_ui_sha256": UI_HASH,
        "ui_source_unchanged": True,
        "rust_library_sha256": hashlib.sha256(dll).hexdigest(),
    }
    items = [(name, source_path.read_bytes()) for name, source_path in pack.stream_files(UPSTREAM)]
    items += [
        ("Serpentine3D/serpentine3d/core/native_spatial.py", b"bridge"),
        ("Serpentine3D/serpentine3d/_native/serpentine_pick_index.dll", source),
        ("Start-Windows.cmd", b"entrypoint"),
        ("launchers/Start-Windows.ps1", b"backend"),
        ("PROVENANCE.json", json.dumps(spec).encode("utf-8")),
    ]
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data in items:
            if name == omit:
                continue
            if name == "Serpentine3D/serpentine3d/ui/__init__.py" and change_ui:
                data += b"\n# altered without authorization\n"
            archive.writestr(prefix + name, data)
        if extra is not None:
            archive.writestr(prefix + extra, b"unapproved file")


def test_complete_kit_validates_original_ui(tmp_path):
    archive = tmp_path / "valid.zip"
    make_archive(archive)
    result = pack.verify(archive)
    assert result["ui_source_unchanged"]
    assert result["members"] >= 64


def test_one_ui_byte_is_enough_to_reject_a_modified_zip(tmp_path):
    archive = tmp_path / "edited.zip"
    make_archive(archive, change_ui=True)
    with pytest.raises(ValueError, match="not byte-identical"):
        pack.verify(archive)


def test_native_dll_tampering_is_detected(tmp_path):
    archive = tmp_path / "bad-dll.zip"
    make_archive(archive, change_dll=True)
    with pytest.raises(ValueError, match="DLL differs"):
        pack.verify(archive)


@pytest.mark.parametrize("extra", ["../bad.txt", "notes/private.ttf", "textures/hidden.woff2"])
def test_archive_rejects_traversal_and_font_files(tmp_path, extra):
    archive = tmp_path / "bad-entry.zip"
    make_archive(archive, extra=extra)
    with pytest.raises(ValueError, match="Invalid archive path or font"):
        pack.verify(archive)


def test_kit_requires_double_click_launcher(tmp_path):
    archive = tmp_path / "no-launcher.zip"
    make_archive(archive, omit="Start-Windows.cmd")
    with pytest.raises(ValueError, match="Missing critical"):
        pack.verify(archive)
