# Serpentine3D: original UI, backend performance experiment

Pinned upstream: `chisomobanzi/Serpentine3D` at
`5a801e0db0ff6bf1d0738430fd9784db4ebfc9b0` (MIT; see UPSTREAM-LICENSE).
This is the actual upstream Qt application, not the separate Arclet egui UI.
Only three backend files are patched. 64 UI/asset files are checksum protected.

## Implemented

- Release tessellation locks and their native shapes when no worker holds them;
  do not evict a still-referenced lock before a waiting worker acquires it.
- Build triangle-picking indexes without the full N-by-3-by-3 corner copy;
  gather corners in bounded blocks and form spatial keys one axis at a time.
- Avoid unnecessary selection-index scratch arrays when keeping all/one chunk;
  reuse the cumulative-sum buffer for other masks.
- Invalidate display bounds/picking caches when public coordinates change.
- Drop old native bounds/meshes when replacing a shape, including dependents.

No UI files, commands, modelling tolerances, or kernel algorithms are changed.

## Reproduce

```sh
git clone https://github.com/chisomobanzi/Serpentine3D.git upstream
git -C upstream checkout --detach 5a801e0db0ff6bf1d0738430fd9784db4ebfc9b0
python scripts/prepare.py upstream --check
python scripts/prepare.py upstream --report evidence/ui-preservation.json
cd upstream
uv sync --extra dev --locked --python 3.12
uv run pytest tests/ -q -n 2 --ignore=tests/e2e_dod.py
uv run serp3d
```

Run the commands from this experiment directory. The prepare script is
idempotent and refuses mixed versions, modified UI, or a modified patch.
Dependencies are installed only in the separate upstream environment.

## Evidence and limits

Local regression tests: 31 passed. Before the changes, 24 of the new contracts
failed (including checks for the newly introduced indexed API); this is not a
claim of 24 independent product bugs.

On one Linux/Python 3.13.5/NumPy 2.3.5 host, three runs of the same seeded
300,000-triangle workload reduced traced peak index allocation from 39,957,184
to 13,557,212 bytes. Median index construction time was 0.1181 to 0.0904 seconds.
All index hashes and gather-result hashes matched. A 32-shape release test
retained 32 native Python wrappers before and zero after. These microbenchmarks
are not whole-app RAM/FPS claims, nor a comparison against Rhino.

CI separately exercises baseline/patched upstream tests on Linux and Windows,
plus real Qt/OpenGL desktop actions and same-host screenshots on Linux. Inspect
the actual run results: defining a workflow is not evidence that it passed.
No subagents were launched. This is not feature parity with Rhino, a security
sandbox, or certification for the owner's Intel GPU. Keep backups of CAD work.
