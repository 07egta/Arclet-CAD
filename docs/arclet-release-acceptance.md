# Arclet CAD Windows and Web release acceptance

This document is the release gate for the separate Arclet binary. It does
not certify Melt3D, SolidCraft Classic, YURACAD or Rhino compatibility.

## Evidence levels (never conflate them)

1. Compiled: locked Cargo and Trunk builds produced a Windows PE and Web assets.
2. Automated CI smoke: Arclet app and native .sc save/reopen tests passed;
   the Windows GUI drew real screenshots; Chromium ran WebGL2/WebGPU and
   browser upload/download tests. Runner GPU support is not guaranteed.
3. Interactive acceptance: a person performed all steps below on a real
   Windows desktop and browser and recorded hashes, versions and screenshots.
   A green GitHub-hosted runner is not proof of full editing acceptance.
4. Release: after required checks and interactive acceptance evidence are
   reviewed, an Arclet-only GitHub Release draft may be published separately
   as a prerelease. CI does not automatically publish anything.

## Windows desktop acceptance (human)

Record commit SHA, Actions run URL, ZIP SHA-256, Windows version, GPU model,
driver, display resolution/DPI, tester and date.

1. Download the arclet-windows-candidate artifact from a successful Actions
   run. Check the ZIP hash using Get-FileHash -Algorithm SHA256.
   Extract the ZIP and launch arclet.exe (not meltcad.exe or solidcraft.exe).
2. Confirm a real Arclet CAD window and an interactive 3D viewport.
   Capture a screenshot. Record whether DirectX 12, Vulkan or GL worked.
   Test an Intel integrated GPU separately if possible.
3. Create a new document. Add a Box, change one dimension, and observe the
   geometry update. Save As arclet-manual-smoke.sc; verify file size > 0.
4. Fully close Arclet. Relaunch it and Open the saved .sc file. Check both
   geometry and edited dimensions. Edit again, Save, fully quit and reopen
   again. Confirm the second edit persisted.
5. Export the object as STL; ensure the nonempty file reimports. Also test
   Undo, Redo and unsaved-change close warning.
6. On failure retain screenshot and
   %LOCALAPPDATA%\Arclet\logs\arclet-startup.log, redacting sensitive paths.
   Verify no other CAD app's files or user settings were altered.

## Browser acceptance (human)

Record browser version, OS, adapter/driver, backend, URL, date, screenshots,
download name/checksum, JS console errors and any GPU warnings.

1. Serve the Trunk build over HTTPS (or localhost HTTP).
2. Open normally with WebGPU and with ?webgl to require WebGL2. Confirm
   Arclet draws a viewport, instead of an indefinitely loading overlay.
3. Create and modify a solid, download a nonempty .sc using Save As, open
   that downloaded file and verify the changed geometry.
4. Edit again and download a fresh copy, then refresh the page and re-upload.
   Browser Save creates a download and does NOT overwrite the old file or
   guarantee persistent in-browser storage. Test another browser or GPU.
5. Capture evidence for every backend and file operation.

The Chromium smoke in scripts/ci/arclet-web-smoke.mjs checks startup,
rendering, download and upload using a real browser, but does not certify
the edited solid or file contents. The .sc geometry roundtrip is separately
tested by a Rust integration test. Treat either failure as FAIL, not PASS.

## CI, artifacts and packaging

- The Arclet-only workflow is .github/workflows/arclet-release.yml.
- Windows uses cargo test --profile ci --locked -p arclet and
  cargo build --release --locked -p arclet.
- Native GUI screenshots run via scripts/ci/arclet-windows-smoke.ps1.
- Web uses Trunk in apps/arclet; output is dist/arclet-web, never dist/web.
- The Windows portable ZIP includes arclet.exe, installation instructions,
  README, licenses, NOTICE, attribution, third-party notices, and checksums.
- ZIP output is an unsigned development build, not a signed installer.
- CI artifacts are build/test CANDIDATES, not verified public releases.
  Logs and screenshots are evidence, not final verification.
- The original THIRD-PARTY-LICENSES.txt may be stale per LICENSE_AUDIT.md.
  Resolve the legal notice audit before asserting licence completeness.

## Release gate

After merging the workflow into main and completing interactive Windows and
browser checklists, manually dispatch the workflow from main with both
create_draft and acceptance_approved selected, and include a reference
to the recorded acceptance evidence. The workflow rechecks all gates from
that exact commit. Success may create an UNPUBLISHED DRAFT Release under
a unique arclet-v...-rc... tag. Publication is a separate human step.

No PR, branch push, failed or blocked job, missing acceptance, or unverified
browser/Windows result may produce a formally published Arclet release.
Never write to existing latest, melt3d-* or YURACAD release tags.
If GitHub Actions does not start due to billing or runner restrictions,
record BLOCKED, not PASS.

## Acceptance evidence template

- Commit / CI workflow URL:
- Windows ZIP name / checksum:
- Windows version / GPU / driver / DPI:
- GUI open / create / edit / save / reopen twice / export results:
- Web browser / GPU / backend:
- WebGPU / WebGL2 / download / upload / screenshots:
- Logs or crashes:
- Tester / date / reviewer:
- Decision: PASS / FAIL / BLOCKED (and details):
