#!/usr/bin/env python3
"""Package the actual PE dependency closure, notices and corresponding OCCT source.

Run on Windows after native tests pass. Requires pefile. System DLLs and
standalone font files are never copied from the builder into the distribution.
"""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import urllib.request

from archive_utils import archive_tree, sha256_file, FONT_SUFFIXES


def copy_text_notices(source: Path, destination: Path) -> int:
    count = 0
    if not source.exists():
        return count
    for path in sorted(source.rglob('*')):
        if not path.is_file() or path.is_symlink() or path.stat().st_size > 2_000_000:
            continue
        name = path.name.upper()
        if not any(key in name for key in ('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT')):
            continue
        if path.suffix.lower() in FONT_SUFFIXES | {'.exe', '.dll'}:
            continue
        try:
            path.read_text(encoding='utf-8')
        except (UnicodeError, OSError):
            continue
        target = destination / path.relative_to(source)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, target)
        count += 1
    return count


def main(prefix: Path, exe: Path, output: Path) -> None:
    import pefile
    source = Path(__file__).resolve().parent.parent
    if output.exists() or output.with_suffix('.zip').exists():
        raise RuntimeError(f'Refusing to replace existing package: {output}')
    output.mkdir(parents=True)
    dlls = {}
    for folder in (prefix / 'Library' / 'bin', prefix):
        for path in sorted(folder.glob('*.dll')):
            dlls[path.name.casefold()] = path
    records = [json.loads(p.read_text()) for p in sorted((prefix / 'conda-meta').glob('*.json'))]
    owners = {}
    for record in records:
        for file in record.get('files', []):
            owners[file.replace('\\', '/').casefold()] = record
    queue = [exe]
    visited = set()
    packages = {}
    while queue:
        path = queue.pop()
        key = path.name.casefold()
        if key in visited:
            continue
        visited.add(key)
        shutil.copy2(path, output / path.name)
        if path != exe:
            relative = path.relative_to(prefix).as_posix().casefold()
            owner = owners.get(relative)
            if owner is None:
                raise RuntimeError(f'Cannot identify distribution license owner: {relative}')
            packages[owner['name']] = owner
        image = pefile.PE(str(path), fast_load=True)
        try:
            image.parse_data_directories(directories=[1, 13])
            entries = list(getattr(image, 'DIRECTORY_ENTRY_IMPORT', []))
            entries += list(getattr(image, 'DIRECTORY_ENTRY_DELAY_IMPORT', []))
            for entry in entries:
                name = entry.dll.decode('ascii').casefold()
                if name in dlls:
                    queue.append(dlls[name])
                elif name.startswith(('api-ms-win-', 'ext-ms-win-')) or (Path(os.environ['SystemRoot']) / 'System32' / name).exists():
                    pass
                else:
                    raise RuntimeError(f'Unresolved runtime dependency {name} imported by {path.name}')
        finally:
            image.close()
    notices = output / 'third-party'
    notices.mkdir()
    for name, record in sorted(packages.items()):
        dest = notices / f"{name}-{record['version']}"
        dest.mkdir()
        (dest / 'package-record.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
        cache = Path(record.get('link', {}).get('source', ''))
        if not (cache / 'info').is_dir():
            raise RuntimeError(f'Missing package provenance/licenses for {name}: {cache}')
        for folder in ('licenses', 'recipe'):
            if (cache / 'info' / folder).exists():
                shutil.copytree(cache / 'info' / folder, dest / folder)
        copied = copy_text_notices(cache / 'info', dest / 'notices')
        if copied == 0 and not (dest / 'licenses').exists():
            raise RuntimeError(f'No license text found for bundled package: {name}')
    occt = packages.get('occt')
    if not occt or occt['version'] != '7.8.1':
        raise RuntimeError('This release recipe is deliberately pinned to OCCT 7.8.1')
    source_url = 'https://github.com/Open-Cascade-SAS/OCCT/archive/refs/tags/V7_8_1.tar.gz'
    archive = notices / 'OCCT-V7_8_1-source.tar.gz'
    urllib.request.urlretrieve(source_url, archive)
    source_record = {'url': source_url, 'sha256': sha256_file(archive),
                     'recipe': f"occt-{occt['version']}/recipe", 'modifications_by_this_application': 'none'}
    (notices / 'occt-corresponding-source.json').write_text(json.dumps(source_record, indent=2), encoding='utf-8')
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1', '--features', 'desktop', '--filter-platform', 'x86_64-pc-windows-msvc'], cwd=source))
    rust_index = []
    for package in metadata['packages']:
        if package.get('source') is None:
            continue
        root = Path(package['manifest_path']).parent
        dest = notices / 'rust' / f"{package['name']}-{package['version']}"
        count = copy_text_notices(root, dest)
        rust_index.append({'name': package['name'], 'version': package['version'], 'license': package.get('license'), 'repository': package.get('repository'), 'notice_files': count})
    (notices / 'rust-dependencies.json').write_text(json.dumps(rust_index, indent=2), encoding='utf-8')
    source_out = output / 'source'
    source_out.mkdir()
    for name in ('src', 'native', 'tests', 'scripts', 'examples'):
        shutil.copytree(source / name, source_out / name, ignore=shutil.ignore_patterns('__pycache__', '*.pyc'))
    for name in ('Cargo.toml', 'Cargo.lock', 'build.rs', 'rustfmt.toml', 'README.md', 'QA.md', 'LICENSE'):
        shutil.copy2(source / name, source_out / name)
    for name in ('LICENSE', 'README.md', 'QA.md'):
        shutil.copy2(source / name, output / name)
    shutil.copytree(source / 'evidence' / 'showcase', output / 'samples')
    (output / 'Start-Arclet.cmd').write_text('@echo off\ncd /d "%~dp0"\narclet-occt-studio.exe --desktop --demo\nif errorlevel 1 pause\n', encoding='utf-8')
    (output / 'RUNTIME-NOTICE.txt').write_text(
        'Development build, unsigned. Extract the entire directory, then run Start-Arclet.cmd.\n'
        'Uses Open CASCADE Technology (LGPL-2.1 with the OCCT exception), dynamically linked.\n'
        'The application source, dependency notices, exact package recipes and OCCT source are included.\n'
        'You may replace compatible DLLs and rebuild the application; no anti-relinking restriction applies.\n'
        'Requires a working OpenGL 3.3 driver. Native core tests do not certify every GPU.\n'
        'This legacy egui experiment is NOT the original Serpentine3D UI.\n'
        'Keep backups. Do not open untrusted CAD files on sensitive systems.\n', encoding='utf-8')
    manifest = {'executable': exe.name, 'runtime_files': sorted(visited), 'packages': {k: {'version': v['version'], 'build': v['build'], 'license': v.get('license')} for k, v in sorted(packages.items())}}
    (output / 'runtime-manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    hashes = {p.relative_to(output).as_posix(): sha256_file(p) for p in sorted(output.rglob('*')) if p.is_file()}
    (output / 'SHA256SUMS.json').write_text(json.dumps(hashes, indent=2), encoding='utf-8')
    archive_tree(output, output.with_suffix('.zip'))
    print(json.dumps(manifest, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--prefix', type=Path, required=True)
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    main(args.prefix.resolve(), args.exe.resolve(), args.output.resolve())
