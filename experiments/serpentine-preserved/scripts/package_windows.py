#!/usr/bin/env python3
"""Build a compact Windows x64 source kit with the *unchanged* Serpentine3D Qt UI.

The launcher prepares a locked, isolated Python runtime on first use. This
build does not bundle third-party wheels, PySide6, OCCT or licensed fonts.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import subprocess
import sys
import tempfile
import zipfile

EXPERIMENT = Path(__file__).resolve().parent.parent
KIT = "Serpentine3D-OriginalUI"
FORBIDDEN = {'.ttf', '.otf', '.ttc', '.woff', '.woff2', '.dfont', '.pfb', '.pfm'}
SKIP = {'.git', '.venv', '__pycache__', '.pytest_cache', '.mypy_cache', '.ruff_cache', '.DS_Store'}
UPSTREAM_TOP = ('LICENSE', 'README.md', 'pyproject.toml', 'uv.lock')
CRITICAL = {
    f'{KIT}/Serpentine3D/serpentine3d/app.py',
    f'{KIT}/Serpentine3D/serpentine3d/launcher.py',
    f'{KIT}/Serpentine3D/serpentine3d/core/native_spatial.py',
    f'{KIT}/Serpentine3D/serpentine3d/_native/serpentine_pick_index.dll',
    f'{KIT}/Serpentine3D/uv.lock',
    f'{KIT}/launchers/Start-Windows.ps1',
    f'{KIT}/Start-Windows.cmd',
}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def stream_files(root: Path):
    for name in UPSTREAM_TOP:
        path = root / name
        if not path.is_file() or path.is_symlink():
            raise ValueError(f'Missing or linked pinned metadata: {path}')
        yield f'Serpentine3D/{name}', path
    for folder in ('serpentine3d', 'assets'):
        for path in sorted((root / folder).rglob('*')):
            if any(part in SKIP for part in path.relative_to(root).parts):
                continue
            if path.is_symlink():
                raise ValueError(f'No symlinks in distributable source: {path}')
            if not path.is_file() or path.suffix.lower() in ('.pyc', '.pyo'):
                continue
            if path.suffix.lower() in FORBIDDEN:
                raise ValueError(f'Bundled fonts are not distributed: {path}')
            yield f'Serpentine3D/{path.relative_to(root).as_posix()}', path


def build(upstream: Path, output: Path) -> dict:
    upstream = upstream.resolve(strict=True)
    # Avoid packaging arbitrary clones, edited UI, partial patches or wrong ABI.
    sys.path.insert(0, str(EXPERIMENT / 'scripts'))
    from prepare import prepare
    report = prepare(upstream, check_only=True)
    if report['state'] != 'patched' or not report['ui_byte_identical_to_upstream']:
        raise ValueError('The exact reviewed backend and original UI are required')
    pinned = json.loads((EXPERIMENT / 'upstream.json').read_text(encoding='utf-8'))
    actual = subprocess.check_output(['git', '-C', str(upstream), 'rev-parse', 'HEAD'],
                                     text=True).strip()
    if actual != pinned['commit']:
        raise ValueError('Unexpected upstream Git revision')
    bin_dir = upstream / 'serpentine3d' / '_native'
    build_info = json.loads((bin_dir / 'build.json').read_text(encoding='utf-8'))
    if build_info.get('library') != 'serpentine_pick_index.dll' or build_info.get('abi') != 1:
        raise ValueError('Expected the tested x64 Windows Rust accelerator')
    dll = bin_dir / build_info['library']
    if digest(dll.read_bytes()) != build_info['sha256']:
        raise ValueError('The Rust library does not match its build manifest')
    extras = [
        ('Start-Windows.cmd', EXPERIMENT / 'launchers' / 'Start-Windows.cmd'),
        ('Start-Python-Backend.cmd', EXPERIMENT / 'launchers' / 'Start-Python-Backend.cmd'),
        ('launchers/Start-Windows.ps1', EXPERIMENT / 'launchers' / 'Start-Windows.ps1'),
        ('source/native-pick-index/Cargo.toml', EXPERIMENT / 'native-pick-index' / 'Cargo.toml'),
        ('source/native-pick-index/Cargo.lock', EXPERIMENT / 'native-pick-index' / 'Cargo.lock'),
        ('source/native-pick-index/src/lib.rs', EXPERIMENT / 'native-pick-index' / 'src' / 'lib.rs'),
        ('source/native-pick-index/LICENSE', EXPERIMENT / 'native-pick-index' / 'LICENSE'),
        ('source/native-pick-index/RUST-LICENSE-MIT', EXPERIMENT / 'native-pick-index' / 'RUST-LICENSE-MIT'),
    ]
    sources = list(stream_files(upstream)) + extras
    names = [f'{KIT}/{name}' for name, _ in sources]
    if len(names) != len(set(names)) or not CRITICAL.issubset(names):
        raise ValueError('The source kit has duplicate or missing critical files')
    info = {
        'kit': KIT,
        'upstream': pinned['repository'],
        'upstream_commit': actual,
        'protected_ui_files': report['protected_ui_files'],
        'protected_ui_sha256': report['protected_ui_sha256'],
        'ui_source_unchanged': True,
        'rust_library_sha256': build_info['sha256'],
        'rust_compiler': build_info['compiler'],
        'native_abi': build_info['abi'],
        'python_runtime': '3.12 (fetched on first launch; not bundled)',
        'dependencies': 'uv.lock (installed on first launch; not bundled)',
        'contents': 'Original Serpentine3D UI, five modified backend modules, optional Rust accelerator',
    }
    instructions = (
        'Serpentine3D ORIGINAL UI / Windows x64 development kit\n'
        '=====================================================\n'
        '1. Extract the whole ZIP to a writable folder. Do not run inside ZIP.\n'
        '2. Double-click Start-Windows.cmd. First run requires internet to\n'
        '   install locked Python/CAD dependencies inside this folder.\n'
        '3. Start-Python-Backend.cmd runs the independent NumPy fallback.\n'
        '4. For diagnostics run: Start-Windows.cmd -SelfTest\n'
        'This is an experimental, source-based preview; not Rhino parity or\n'
        'a polished installer. Keep backups of your design files.\n'
        'Serpentine3D UI/assets remain byte-identical to the pinned upstream.\n'
        'Original MIT license: Serpentine3D/LICENSE. Rust licenses: source/.\n'
        'No fonts, third-party CAD wheels, credentials or user data bundled.\n'
    ).encode('utf-8')
    sources += [('README-FIRST.txt', instructions),
                ('PROVENANCE.json', (json.dumps(info, indent=2) + '\n').encode('utf-8'))]
    output.parent.mkdir(parents=True, exist_ok=True)
    temp = None
    try:
        with tempfile.NamedTemporaryFile(prefix='.serp-build-', suffix='.zip',
                                         dir=output.parent, delete=False) as stream:
            temp = Path(stream.name)
        with zipfile.ZipFile(temp, 'w', allowZip64=True, compression=zipfile.ZIP_DEFLATED,
                             compresslevel=6) as archive:
            for name, source in sorted(sources, key=lambda pair: pair[0]):
                arcname = f'{KIT}/{name}'
                if isinstance(source, Path):
                    if source.is_symlink() or source.suffix.lower() in FORBIDDEN:
                        raise ValueError(f'Forbidden source file: {source}')
                    data = source.read_bytes()
                else:
                    data = source
                item = zipfile.ZipInfo(arcname, (1980, 1, 1, 0, 0, 0))
                item.create_system = 3
                item.external_attr = (stat.S_IFREG | 0o644) << 16
                item.compress_type = zipfile.ZIP_STORED if arcname.lower().endswith(
                    ('.dll', '.png', '.jpg', '.jpeg', '.webp', '.zip', '.ico')) else zipfile.ZIP_DEFLATED
                archive.writestr(item, data)
        verify(temp)
        os.replace(temp, output)
        return {'path': str(output), 'size_bytes': output.stat().st_size,
                'sha256': digest(output.read_bytes()), 'ui_preservation': report}
    finally:
        if temp is not None:
            temp.unlink(missing_ok=True)


def verify(archive: Path) -> dict:
    with zipfile.ZipFile(archive, 'r') as z:
        names = z.namelist()
        if len(names) != len(set(names)) or not CRITICAL.issubset(names):
            raise ValueError('Missing critical source-kit components')
        if any(PurePosixPath(n).is_absolute() or '..' in PurePosixPath(n).parts
               or not n.startswith(f'{KIT}/') or Path(n).suffix.lower() in FORBIDDEN
               for n in names):
            raise ValueError('Invalid archive path or font file')
        if z.testzip() is not None:
            raise ValueError('Corrupted ZIP member')
        info = json.loads(z.read(f'{KIT}/PROVENANCE.json'))
        native_bytes = z.read(f'{KIT}/Serpentine3D/serpentine3d/_native/serpentine_pick_index.dll')
        if digest(native_bytes) != info['rust_library_sha256']:
            raise ValueError('The packaged Rust DLL differs from the original build')
        prefix = f'{KIT}/Serpentine3D/'
        ui_names = [n for n in names if n in {
                        prefix + 'serpentine3d/app.py',
                        prefix + 'serpentine3d/launcher.py',
                        prefix + 'serpentine3d/__main__.py'} or n.startswith(
                            (prefix + 'serpentine3d/ui/', prefix + 'assets/'))]
        pairs = [(name[len(prefix):], digest(z.read(name))) for name in sorted(ui_names)]
        fingerprint = digest(json.dumps(pairs, ensure_ascii=True,
                                        separators=(',', ':')).encode())
        if (len(pairs) != info['protected_ui_files']
                or fingerprint != info['protected_ui_sha256']
                or not info['ui_source_unchanged']):
            raise ValueError('The packaged Serpentine3D UI is not byte-identical')
        return {'members': len(names), 'size_bytes': archive.stat().st_size,
                'rust_library_sha256': info['rust_library_sha256'],
                'ui_source_unchanged': True, 'upstream_commit': info['upstream_commit']}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('upstream', nargs='?', type=Path)
    p.add_argument('--output', type=Path)
    p.add_argument('--verify', type=Path)
    args = p.parse_args()
    if args.verify:
        print(json.dumps(verify(args.verify), indent=2))
        return
    if args.upstream is None or args.output is None:
        p.error('Build requires an upstream checkout and --output')
    print(json.dumps(build(args.upstream, args.output), indent=2))


if __name__ == '__main__':
    main()
