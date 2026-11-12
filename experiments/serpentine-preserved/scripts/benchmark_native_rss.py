#!/usr/bin/env python3
"""Measure native and NumPy allocations with process RSS, not tracemalloc.

Rust's allocator is invisible to Python tracemalloc. Each sample therefore
runs in a fresh Linux process with the same pre-generated input, reporting OS
peak resident memory, RSS after input loading, and exact output-array hashes.
Timing is recorded, not used as a CI correctness gate.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

WORKER = r'''
import gc, hashlib, json, os, resource, sys, time
import numpy as np
from serpentine3d.core import spatial
folder, backend = sys.argv[1:]
vertices = np.load(os.path.join(folder, 'vertices.npy'))
triangles = np.load(os.path.join(folder, 'triangles.npy'))
if backend != 'baseline':
    from serpentine3d.core.native_spatial import backend_status
    status = backend_status()
    if backend == 'rust' and not status['available']:
        raise RuntimeError(status)
else:
    status = {'available': False, 'requested': 'original NumPy'}
gc.collect()
def rss():
    with open('/proc/self/status') as f:
        for line in f:
            if line.startswith('VmRSS:'):
                return int(line.split()[1]) * 1024
before = rss()
previous_peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024
start = time.perf_counter()
if backend == 'baseline':
    result = spatial.build_index(vertices[triangles])
else:
    result = spatial.build_triangle_index(vertices, triangles)
elapsed = time.perf_counter() - start
peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024
after = rss()
hash = hashlib.sha256()
for array in (result.order, result.starts, result.mins, result.maxs):
    hash.update(memoryview(array).cast('B'))
print(json.dumps({'backend': backend, 'library': status, 'triangles': len(triangles),
 'python': sys.version.split()[0], 'numpy': np.__version__, 'seconds': elapsed,
 'spatial_source': spatial.__file__,
 'rss_before_bytes': before, 'rss_after_bytes': after,
 'prior_high_water_bytes': previous_peak, 'process_peak_rss_bytes': peak,
 'peak_minus_loaded_rss_bytes': max(0, peak - before), 'index_sha256': hash.hexdigest()}))
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--patched', type=Path, required=True)
    parser.add_argument('--triangles', type=int, default=3000000)
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != 'linux':
        parser.error('This RSS probe uses Linux /proc and resource units')
    if args.triangles < 20000 or args.repeats < 1:
        parser.error('Use at least 20,000 triangles and one repeat')
    rows = []
    with tempfile.TemporaryDirectory(prefix='serp-index-input-') as folder:
        root = Path(folder)
        n = args.triangles
        vertices = np_open(root / 'vertices.npy', 'float32', (n * 3, 3))
        triangles = np_open(root / 'triangles.npy', 'uint32', (n, 3))
        import numpy as np
        rng = np.random.default_rng(20261112)
        for start in range(0, n, 65536):
            stop = min(n, start + 65536)
            centre = rng.uniform(-500, 500, (stop - start, 1, 3)).astype(np.float32)
            centre[:, :, 2] *= .01
            points = centre + rng.uniform(-.4, .4, (stop - start, 3, 3)).astype(np.float32)
            vertices[start * 3:stop * 3] = points.reshape(-1, 3)
            triangles[start:stop] = np.arange(start * 3, stop * 3, dtype=np.uint32).reshape(-1, 3)
        vertices.flush()
        triangles.flush()
        del vertices, triangles
        # Rotate order to avoid comparing only a cold baseline and warm native.
        modes = ['baseline', 'python', 'rust']
        for repeat in range(args.repeats):
            order = modes[repeat % 3:] + modes[:repeat % 3]
            for backend in order:
                env = os.environ.copy()
                env['PYTHONPATH'] = str((args.baseline if backend == 'baseline' else args.patched).resolve())
                env['SERP3D_PICK_BACKEND'] = 'python' if backend == 'baseline' else backend
                result = subprocess.run([sys.executable, '-c', WORKER, folder, backend], env=env,
                                        cwd=root, capture_output=True, text=True, check=True, timeout=120)
                row = json.loads(result.stdout)
                expected = (args.baseline if backend == 'baseline' else args.patched).resolve() / 'serpentine3d/core/spatial.py'
                if Path(row['spatial_source']).resolve() != expected:
                    raise AssertionError('Benchmark imported a different checkout')
                row['repeat'] = repeat
                rows.append(row)
                print(json.dumps(row), flush=True)
    import statistics
    summary = {}
    for mode in modes:
        data = [r for r in rows if r['backend'] == mode]
        summary[mode] = {'median_seconds': statistics.median(r['seconds'] for r in data),
                         'max_observed_incremental_peak_rss_bytes': max(r['peak_minus_loaded_rss_bytes'] for r in data)}
    report = {'measurement': 'Fresh-process Linux OS RSS, including native allocations; not whole-app RAM',
              'triangles': args.triangles, 'repeats': args.repeats, 'seed': 20261112,
              'same_output_arrays': len({r['index_sha256'] for r in rows}) == 1,
              'summary': summary, 'samples': rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(summary, indent=2))
    if not report['same_output_arrays']:
        raise AssertionError('Native and reference index hashes differ')


def np_open(path, dtype, shape):
    import numpy as np
    return np.lib.format.open_memmap(path, mode='w+', dtype=dtype, shape=shape)


if __name__ == '__main__':
    main()
