# C/C++ library survey for Arclet CAD

Date of survey: 2026-11-02. Licences were read from each project's README, LICENSE page or the licence text as returned by the web fetch on that date. Where a licence text could not be retrieved, the row says so.

Scope: candidates for a Rust desktop NURBS CAD (Arclet CAD, MIT OR Apache-2.0) that would be linked as a dependency.

Project rule applied: no GPL, LGPL or AGPL code, and no reading of its source. For those projects only the licence page was consulted, and they are marked AVOID.

Verdict key:
- USE: permissive licence, linkable statically or dynamically with notices.
- USE-DYNAMIC-ONLY: reserved for weak-copyleft code that may be linked only as a separate shared library. No candidate reaches this verdict under the project rule, so none is marked with it.
- AVOID: copyleft, commercial-only, licence not verified, or otherwise unsuitable.

## Summary table

| # | Library | Licence (SPDX) | Copyleft | Static / dynamic link into MIT/Apache binary | Language / build | Rust binding | What it gives Arclet | Verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | openNURBS | UNVERIFIED: the repository LICENSE has no explicit permission grant; the Rhino developer page says MIT | None | Both fine, keep the notice | C++, CMake and VS/Xcode | None found in this survey (rhino3dm exists for JS and Python, not Rust) | 3DM read/write, NURBS evaluation | USE |
| 2 | OpenCASCADE (OCCT) | LGPL-2.1 with OCCT exception | Weak copyleft | Project rule: not allowed | C++, CMake | `cadrum` and `chijin` wrap OCCT but bundle it (LGPL) | B-rep kernel, STEP/IGES | AVOID |
| 3 | CGAL | GPL / LGPL / commercial (per-package; not resolved here) | Strong (GPL parts) | Project rule: not allowed | C++, CMake | None checked | Geometry algorithms | AVOID |
| 4 | Manifold | Apache-2.0 | None | Both fine | C++, CMake | `manifold3d` 0.4.0 (Apache-2.0 OR MIT), `manifold-csg` 0.4.1 | Guaranteed-manifold mesh booleans | USE |
| 5 | Clipper2 | BSL-1.0 (Boost) | None | Both fine | C++ (also C#, Delphi) | `clipper2` 0.6.0, `clipper2-rust` 1.1.0 (licences of crates not individually checked) | 2D polygon boolean and offset | USE |
| 6 | Eigen | MPL-2.0 | File-level weak copyleft | Both fine; modified Eigen files stay MPL-2.0 | C++ header-only, CMake | Rust-native alternative: nalgebra (not checked in this run) | Dense linear algebra | USE |
| 7 | libigl | MPL-2.0 core, but repo also contains GPL-3.0 files | GPL parts present | Project rule: not allowed as a whole repo | C++, CMake | None checked | Geometry processing | AVOID |
| 8 | Ceres Solver | BSD-3-Clause (main), Apache-2.0 and MIT for some parts in the same LICENSE file | None | Both fine, keep notices | C++, CMake and Bazel | None checked | Non-linear least squares (constraint solving) | USE |
| 9 | NLopt | LGPL-2.1-or-later by default; MIT only if built without the `luksan` code | LGPL by default | Project rule: default build not allowed | C with bindings incl. Rust, CMake | `nlopt` crates exist (licence not checked) | Non-linear optimisation | AVOID |
| 10 | SISL | AGPL-3.0 | Strong (network clause) | Not allowed | C, CMake | None checked | NURBS library | AVOID |
| 11 | TinyNURBS | Not verified (repo page not retrieved; licence unconfirmed) | Unknown | Cannot assess | C++ | None | NURBS curves and surfaces | AVOID (until licence is read) |
| 12 | Geometric Tools (GTE) | BSL-1.0 (Boost) | None | Both fine | C++14, Visual Studio and CMake | None checked | Math, geometry, graphics; NURBS not confirmed on page | USE |
| 13 | Assimp | BSD-3-Clause (page: "modified, 3-clause BSD") | None | Both fine | C++, CMake | `russimp` (licence not checked) | Mesh import (40+ formats) | USE |
| 14 | STEPcode | BSD-3-Clause (page: "3-clause BSD", text in COPYING not read) | None | Both fine | C++ generated from EXPRESS, CMake | None checked | STEP Part 21 read/write | USE (verify COPYING) |
| 15 | Open Design Alliance SDKs | Commercial, membership-based | Not applicable | Requires paid agreement | C++ | None | DWG/DGN/RVT import | AVOID |
| 16 | Parasolid / ACIS (Siemens / Spatial) | Commercial, per-seat or per-product | Not applicable | Requires paid licence | C/C++ | None | Full B-rep kernel | AVOID |
| 17 | Bullet | zlib | None | Both fine | C++, CMake/Premake | `rapier` is the Rust-native alternative (not checked) | Collision and physics | USE |
| 18 | Embree | Apache-2.0 | None | Both fine | C/C++, CMake | `embree` crates (not checked) | Ray tracing kernels | USE |
| 19 | OpenSubdiv | Licence text not retrieved from page; expected Apache-2.0, verify LICENSE.txt | Unknown until checked | Both fine if Apache-2.0 | C++17, CMake | None checked | Subdivision surface evaluation | USE (verify LICENSE.txt) |
| 20 | Earcut.hpp (mapbox) | ISC | None | Both fine | C++ header-only | n/a (Rust `earcutr` is a port, licence not checked) | Polygon triangulation with holes | USE |
| 21 | libtess2 | SGI Free Software License B 2.0 (text not read here) | Not GPL; read the text before use | Verify the text first | C, Premake/Bazel | None | Polygon tessellation | USE (verify licence text) |
| 22 | CPython (embedding) | PSF-2.0 expected; licence page fetch was blocked, not verified | None (permissive) | Both fine once verified | C | `pyo3` (Apache-2.0 OR MIT, not checked) | Scripting | USE (verify licence page) |
| 23 | Gmsh | GPL-2.0-or-later with the Gmsh exception (exception covers only listed codes) | GPL | Not allowed | C++, CMake | None | Mesh generation | AVOID |
| 24 | MeshLab / VCG | GPL (licence page fetch blocked; treated as GPL under the rule) | GPL | Not allowed | C++ | None | Mesh processing | AVOID |

Note on rows 1 and 12: the openNURBS repository LICENSE file could not be fetched directly, so the MIT verdict rests on the Rhino developer licence page. Check the repository LICENSE before shipping.

## Notes per library

1. **openNURBS.** Licence: MIT, per the Rhino developer licence page, which names "Rhino SDKs and APIs, including openNURBS" and gives the copyright as 1993-2026 Robert McNeel & Associates. The openNURBS README describes it as C++ source for reading and writing 3DM files, with NURBS evaluation tools; it also includes CMake and Visual Studio/Xcode projects. This is the best candidate for 3DM import and export. Rust binding: none found. A Rust-side FFI shim would be needed; wrap only the functions Arclet needs.
   - https://developer.rhino3d.com/license/
   - https://github.com/mcneel/opennurbs
   - https://developer.rhino3d.com/guides/general/rhino-technology-overview/

2. **OpenCASCADE (OCCT).** Licence: LGPL-2.1 with the OCCT exception (OCCT_LGPL_EXCEPTION.txt). Under the project rule this is AVOID, even though the exception would permit dynamic linking. The Rust crates `cadrum` (MIT, bundles OCCT 7.9.3 and builds it from source) and `chijin` also carry OCCT's licence, so they inherit the problem.
   - https://github.com/Open-Cascade-SAS/OCCT
   - https://docs.rs/crate/cadrum/0.4.1

3. **CGAL.** The repository states GPL, LGPL and commercial options by package; the README says the licence is in LICENSE.md. The CGAL licence page did not resolve the per-package split in this run. AVOID under the rule.
   - https://github.com/CGAL/cgal
   - https://www.cgal.org/license.html (fetch was blocked; not read)

4. **Manifold.** Licence: Apache-2.0. Guaranteed-manifold mesh booleans; C++ with CMake, and Python/WASM bindings. Rust: `manifold3d` 0.4.0 is Apache-2.0 OR MIT and wraps the C++ library; `manifold-csg` 0.4.1 is a separate crate. Strong fit for mesh CSG in an MIT/Apache app.
   - https://github.com/elalish/manifold
   - https://docs.rs/crate/manifold3d/latest

5. **Clipper2.** Licence: Boost Software License 1.0 (LICENSE_1_0.txt). 2D polygon boolean ops, offsetting and constrained Delaunay triangulation, which the README flags as buggy. Rust: `clipper2` 0.6.0 and `clipper2-rust` 1.1.0 exist.
   - https://github.com/AngusJohnson/Clipper2
   - https://docs.rs/crate/clipper2-rust/latest/source/README.md

6. **Eigen.** Licence: MPL-2.0 (COPYING.MPL2). File-level copyleft: unmodified use in an MIT/Apache binary is fine; any changed Eigen file must stay MPL-2.0. For a Rust app, nalgebra is the native alternative and avoids the C++ build entirely.
   - https://gitlab.com/libeigen/eigen/-/blob/master/COPYING.MPL2

7. **libigl.** The README says "Simple MPL-2.0-licensed C++ geometry processing library", but the repo also lists GPL-3.0 files and the page does not say which files they cover. Under the rule, AVOID the repo as a whole.
   - https://github.com/libigl/libigl

8. **Ceres Solver.** The LICENSE file opens with the Ceres copyright (Google, 2023) and BSD-3 terms, and also includes the Apache-2.0 full text and an MIT notice for example code derived from libmv. Use the permissive terms; keep all notices. Good fit for constraint solving if Arclet ever needs non-linear least squares.
   - https://github.com/ceres-solver/ceres-solver/blob/master/LICENSE

9. **NLopt.** The COPYING file says the default build is LGPL-2.1-or-later (the `luksan` code). An MIT-licensed build is possible only if `luksan` is excluded. Under the rule, AVOID the default build. Reconsider only after a deliberate decision to build the MIT subset.
   - https://github.com/stevengj/nlopt/blob/master/COPYING

10. **SISL.** Licence: AGPL-3.0 (sidebar). Comprehensive NURBS library in C. AVOID. Note that AGPL is the strongest copyleft in this list.
    - https://github.com/SINTEF-Geometry/SISL

11. **TinyNURBS.** The repository page could not be fetched, and a search did not confirm the licence. AVOID until the licence text is read.

12. **Geometric Tools (GTE).** Licence: Boost Software License 1.0. C++14 math, geometry and graphics engine. The page does not mention NURBS or B-splines, so do not count on it for surface work; it is useful as a math and geometry reference library.
    - https://github.com/davideberly/GeometricTools

13. **Assimp.** Licence: 3-clause BSD ("modified, 3-clause BSD-License"). Imports 40+ mesh formats. Rust: `russimp` exists (licence not checked).
    - https://github.com/assimp/assimp

14. **STEPcode.** Licence: 3-clause BSD (summary on page; COPYING not read). Generates C++ from EXPRESS schemas and reads/writes STEP Part 21. Confirm the generated code carries the same licence before shipping.
    - https://github.com/stepcode/stepcode

15. **Open Design Alliance SDKs.** Commercial; access depends on paid membership and per-SDK agreements. AVOID for an open dependency.
    - https://www.opendesign.com/pricing.md

16. **Parasolid / ACIS.** Commercial kernels, licensed per product. AVOID.
    - https://en.wikipedia.org/wiki/ShapeManager (ACIS origin of Autodesk's kernel)

17. **Bullet.** Licence: zlib, per the repo. Physics and collision; only relevant if Arclet needs collision.
    - https://github.com/bulletphysics/bullet3

18. **Embree.** Licence: Apache-2.0. Ray tracing kernels; relevant only for a renderer or picking acceleration.
    - https://github.com/embree/embree

19. **OpenSubdiv.** The page did not show the licence text; verify LICENSE.txt before use. Subdivision evaluation that matches RenderMan.
    - https://github.com/PixarAnimationStudios/OpenSubdiv

20. **Earcut.hpp.** Licence: ISC. Header-only polygon triangulation with holes; a Rust port exists as `earcutr` (licence not checked).
    - https://github.com/mapbox/earcut.hpp

21. **libtess2.** Licence: SGI Free Software License B 2.0 (text not read here). Polygon tessellator. Read the licence text before use.
    - https://github.com/memononen/libtess2

22. **CPython embedding.** The licence page fetch was blocked; the PSF licence is expected to be permissive, but this row is UNVERIFIED. Verify https://docs.python.org/3/license.html before any use.

23. **Gmsh.** Licence: GPL-2.0-or-later with the Gmsh exception (the SPDX exception page says it permits combining Gmsh with listed codes such as Netgen, METIS, OpenCASCADE and ParaView). Still GPL, so AVOID.
    - https://spdx.org/licenses/Gmsh-exception.html

24. **MeshLab / VCG.** The licence page fetch was blocked; treated as GPL under the project rule. AVOID.
    - https://www.meshlab.net/ (fetch blocked; not read)

## How well-known CAD tools are built (public facts only)

For orientation only. Nothing here is to be copied.

- **Rhino.** Core geometry is in-house C++ code ("Rhino Core"). openNURBS is the free C++ toolkit for reading and writing 3DM files; Rhino uses it natively. The C++ SDK is Windows-only; RhinoCommon is the .NET SDK. Source: https://developer.rhino3d.com/guides/general/rhino-technology-overview/
- **Plasticity.** Runs on the Parasolid kernel (commercial, Siemens). It is proprietary, sold as perpetual licences (Indie and Studio tiers). The Wikipedia page does not name its UI framework, so Electron/three.js is not confirmed. Source: https://en.wikipedia.org/wiki/Plasticity_(software)
- **FreeCAD / OpenCASCADE.** FreeCAD is LGPL-2.1, uses OpenCASCADE as its geometry kernel, and offers a Python API. Source: https://github.com/FreeCAD/FreeCAD ; https://wiki.freecad.org/OpenCASCADE
- **SolveSpace.** GPL-3.0-or-later parametric 2D/3D CAD, written in C, built with CMake. Source: https://github.com/SolveSpace/solvespace
- **Onshape / Parasolid.** Onshape is widely reported to use the Parasolid kernel, and Parasolid is licensed by Siemens. The fetch of Onshape's own page was blocked, so this is from search-result summaries only. Source: https://www.engineering.com/parasolid-d-cubed-and-siemens-the-heart-of-your-cad-software-belongs-to-another/
- **Fusion / ASM.** Autodesk's ShapeManager is its in-house kernel, forked from ACIS 7.0 in 2001 and first shipped in Inventor 5.3 (2002). Whether Fusion uses it is not confirmed on that page. Source: https://en.wikipedia.org/wiki/ShapeManager

## Gaps

- Blocked or not retrieved: CGAL licence page, MeshLab site, CPython licence page, Gmsh GitLab page, Onshape page, FreeCAD and Fusion pages, TinyNURBS repo, openNURBS repo LICENSE file, earcut at nothings/, OpenSubdiv LICENSE.txt, libtess2 licence text.
- Not searched: Rust bindings for Assimp, Embree, OpenSubdiv, Bullet (rapier is the native alternative), and licence text of the Rust crates listed.
- None of the candidates is marked USE-DYNAMIC-ONLY.

## Update: openNURBS licence is unresolved

The repository `LICENSE` (read directly) is a copyright line, a warranty disclaimer and the bundled zlib licence. It contains no explicit grant of permission. The MIT claim rests only on a web page. Until McNeel confirms in writing, treat openNURBS as **not cleared**: do not vendor, link or port it, and do not use it as a source of code. A clean-room 3DM reader based on the public file-format description is the safe alternative.

## Mission 17 linkage policy

The optional native bridge pins Clipper2 and Manifold at exact source revisions, disables optional native dependencies/downloads and documents FFI safety and redistribution obligations in docs/native-cpp-bridge.md. Eigen and Ceres remain unlinked candidates pending workload-specific benchmarks and transitive dependency audits. This update does not change the final unresolved openNURBS verdict: no source or linkage until explicit permission is verified.
