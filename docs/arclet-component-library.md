# Arclet reusable component library (Mission 38)

This feature is a **local catalog of editable source designs**, not a new modeling kernel. It does not change the existing material/appearance `library.*` format.

## Manifest

The library is a JSON file using `arclet-component-library/1`. Each entry contains an ID, positive integer version, human-readable name, category, tags, source .sc path relative to the manifest folder, exact lowercase SHA-256 of the .sc source bytes, source unit, preview bounding-box dimensions and a list of informational parameter names/limits.

Categories: `furniture`, `electronics`, `fasteners`, `plumbing`, `architecture`, `lighting`, `machinery`, `primitives`.

An entry is **immutable per (ID, version)**. To publish an updated source, save it under another filename and add a new version with a new SHA-256. Never silently replace a source used by an older entry. Source paths cannot contain traversal, absolute roots, URI schemes, Windows drive names or backslashes. Files are still checked against the current file-access scope.

Example (make sure to replace the digest with the real hash of the saved file):

```json
{
  "format": "arclet-component-library/1",
  "name": "My local design parts",
  "entries": [
    {
      "id": "chair", "name": "Chair", "category": "furniture",
      "tags": ["seating"], "source": "furniture/chair.sc",
      "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "version": 1, "unit": "mm",
      "preview": {"bounds": [450, 420, 900], "description": "Chair source design"},
      "parameters": [
        {"name": "seat_width", "default": 450, "min": 300, "max": 650, "unit": "mm"}
      ]
    }
  ]
}
```

## Commands

```json
{"command":"components.catalog","params":{"path":"/path/to/parts.json","query":"chair","tags":["seating"]}}
```

Read the returned exact `version` and `sha256` and pass them to insertion:

```json
{"command":"components.insert","params":{"path":"/path/to/parts.json","entry":"chair","version":1,"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","translate":[10,20,0]}}
```

The normal `component.insert_design` implementation owns ID allocation, linked geometry and undo. Translation is in **millimetres**. For now, sources with other units are rejected rather than silently scaled. To create an additional linked instance, set `reuse: true`. Source parametric history remains editable by opening its source .sc design and using the existing edit-in-place/update-link workflow; it is **not** imported as a parametric feature timeline.

## Quality and limitations

- Manifest validation is bounded to 1 MiB and 256 entries; fields and parameter bounds are checked; malformed and unknown fields are rejected.
- Source designs are read with the engine's bounded design-file reader and checked against the exact SHA-256 before inserting.
- The existing insertion command reads the file again, so concurrent replacement between hash verification and insertion remains an unresolved TOCTOU risk.
- This iteration does **not** provide an end-user catalog editor, registration workflow, source-byte caching, preview images, automatic unit conversion or copying source parameter dependencies into the assembly.
- The manifest preview is descriptive metadata, **not** trusted geometry.
- No mass-produced model library or unverified standards-compliance claim is included.

## Verification

The added unit tests cover manifest roundtrip, version ordering, search, invalid paths, malformed JSON, duplicate IDs/tags, invalid parameter values, missing versions and changed source hashes. The engine insertion scenario saves a small block design, catalogs it, inserts it at an offset and checks error paths preserve the document.

These tests have been **written but not executed in a local Rust environment**; CI results should be checked before review or merge. In particular, acceptance requires `cargo test -p solidcraft-io component_library`, `cargo test -p solidcraft-engine component_library`, then repository-required full CI. Verify linked and unlinked workflow with actual chair, table, lamp and fastener designs before declaring Mission 38 complete.
