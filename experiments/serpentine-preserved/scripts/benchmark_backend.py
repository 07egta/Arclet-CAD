"""Reproducible microbenchmarks; wall time is evidence, not a test gate."""
from __future__ import annotations
import argparse
import gc
import hashlib
import json
import platform
import statistics
import time
import tracemalloc
import weakref

import numpy as np


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--triangles', type=int, default=300000)
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    from serpentine3d.core.tessellate import DisplayMesh
    from serpentine3d.core import geometry, scene as scene_mod
    rng = np.random.default_rng(20261112)
    # Small triangles on a sampled sheet are representative of survey meshes.
    centres = rng.uniform(-500, 500, (args.triangles, 1, 3)).astype(np.float32)
    centres[:, :, 2] *= .01
    vertices = (centres + rng.uniform(-.4, .4, (args.triangles, 3, 3)).astype(np.float32)).reshape(-1, 3)
    triangles = np.arange(len(vertices), dtype=np.uint32).reshape(-1, 3)
    del centres
    times, peaks, signatures = [], [], []
    for _ in range(args.repeats):
        mesh = DisplayMesh(local=vertices, triangles=triangles)
        gc.collect()
        tracemalloc.start()
        start = time.perf_counter()
        index = mesh.triangle_index()
        times.append(time.perf_counter() - start)
        peaks.append(tracemalloc.get_traced_memory()[1])
        tracemalloc.stop()
        signatures.append(hashlib.sha256(index.order.tobytes() + index.starts.tobytes() + index.mins.tobytes() + index.maxs.tobytes()).hexdigest())
        del mesh, index
    # Separate gather peak: the already-built index is outside the measurement.
    mesh = DisplayMesh(local=vertices, triangles=triangles)
    index = mesh.triangle_index()
    gather = {}
    for name, step in [('all', 1), ('sparse', 11)]:
        keep = np.zeros(index.count, dtype=bool)
        keep[::step] = True
        tracemalloc.start()
        start = time.perf_counter()
        selected = index.gather(keep)
        elapsed = time.perf_counter() - start
        peak = tracemalloc.get_traced_memory()[1]
        tracemalloc.stop()
        gather[name] = {'seconds': elapsed, 'peak_bytes': peak, 'primitives': len(selected), 'sha256': hashlib.sha256(selected.tobytes()).hexdigest()}
        del selected
    refs = []
    for i in range(32):
        shape = geometry.make_box((i * 2., 0., 0.), 1., 1., 1.)
        refs.append(weakref.ref(shape))
        with scene_mod._tess_lock(shape):
            pass
    del shape
    gc.collect()
    report = {'python': platform.python_version(), 'numpy': np.__version__, 'platform': platform.platform(),
              'triangles': args.triangles, 'seed': 20261112, 'repeats': args.repeats,
              'index_seconds': times, 'index_median_seconds': statistics.median(times),
              'index_traced_peak_bytes': peaks, 'index_peak_bytes': max(peaks),
              'index_sha256': signatures, 'gather': gather,
              'released_shapes_retained_by_lock_registry': sum(r() is not None for r in refs)}
    from pathlib import Path
    Path(args.output).write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(report, indent=2))

if __name__ == '__main__':
    main()
