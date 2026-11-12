#!/usr/bin/env python3
"""Apply the reviewed backend patch only to the pinned upstream source.

No downloads, dependency installation, UI edits, or main-workspace changes.
Already-patched checkouts are accepted; partial/unrelated changes are refused.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

BUNDLE = Path(__file__).resolve().parent.parent


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ui_fingerprint(root: Path) -> tuple[str, int]:
    paths = [root / 'serpentine3d' / name
             for name in ('app.py', 'launcher.py', '__main__.py')]
    for folder in ('serpentine3d/ui', 'assets'):
        paths.extend(p for p in (root / folder).rglob('*')
                     if p.is_file() and '__pycache__' not in p.parts
                     and p.suffix != '.pyc')
    pairs = []
    for path in sorted(paths, key=lambda p: p.relative_to(root).as_posix()):
        if path.is_symlink():
            raise ValueError(f'UI symlink is not a pinned source file: {path}')
        pairs.append((path.relative_to(root).as_posix(), digest(path)))
    payload = json.dumps(pairs, ensure_ascii=True, separators=(',', ':')).encode()
    return hashlib.sha256(payload).hexdigest(), len(pairs)


def prepare(root: Path, *, check_only: bool = False) -> dict:
    root = root.resolve(strict=True)
    spec = json.loads((BUNDLE / 'upstream.json').read_text(encoding='utf-8'))
    patch = BUNDLE / 'patches' / '0001-memory-and-picking.patch'
    if digest(patch) != spec['patch_sha256']:
        raise ValueError('The patch differs from its reviewed checksum')
    ui, count = ui_fingerprint(root)
    if ui != spec['protected_ui_sha256'] or count != spec['protected_ui_files']:
        raise ValueError('UI/assets differ from the pinned upstream; no files changed')
    state = {}
    for relative, hashes in spec['changed_files'].items():
        path = root / relative
        if path.is_symlink() or any(p.is_symlink() for p in path.parents if p != root):
            raise ValueError(f'Symlink in patch destination: {relative}')
        state[relative] = digest(path)
    before = all(state[p] == h['before'] for p, h in spec['changed_files'].items())
    after = all(state[p] == h['after'] for p, h in spec['changed_files'].items())
    if not before and not after:
        raise ValueError('Backend source is mixed or modified; no files changed')
    if before:
        subprocess.run(['git', 'apply', '--check', '--whitespace=error', str(patch)],
                       cwd=root, check=True)
        if not check_only:
            subprocess.run(['git', 'apply', '--whitespace=error', str(patch)],
                           cwd=root, check=True)
    expected = 'before' if check_only and before else 'after'
    for relative, hashes in spec['changed_files'].items():
        if digest(root / relative) != hashes[expected]:
            raise RuntimeError(f'Post-application checksum mismatch: {relative}')
    if ui_fingerprint(root) != (ui, count):
        raise RuntimeError('UI preservation invariant failed')
    return {'upstream': spec['repository'], 'upstream_commit': spec['commit'],
            'state': 'checked-original' if expected == 'before' else 'patched',
            'changed_files': list(spec['changed_files']),
            'protected_ui_files': count, 'protected_ui_sha256': ui,
            'ui_byte_identical_to_upstream': True}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=Path, help='Pinned Serpentine3D checkout')
    parser.add_argument('--check', action='store_true', help='Validate without editing')
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    report = prepare(args.root, check_only=args.check)
    text = json.dumps(report, indent=2)
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(text + '\n', encoding='utf-8')
    print(text)


if __name__ == '__main__':
    main()
